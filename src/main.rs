use anyhow::{Context, Result};
use clap::Parser;
use colored::*;
use csv::Writer;
use reqwest;
use scraper::{ElementRef, Html, Selector};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about = "Extract tables from HTML files and save them as CSV", long_about = None)]
struct Cli {
    /// Input HTML file path or URL
    #[arg(short, long)]
    input: String,

    /// Output directory for CSV files
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,

    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,
}

#[derive(Clone, Debug)]
struct Cell {
    content: String,
    colspan: usize,
    rowspan: usize,
}

//impl std::fmt::Debug for Cell {
//    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//        let content_with_newlines = self.content.replace("\n", "\\n");
//        write!(
//            f,
//            "[{}, {}, {}]",
//            content_with_newlines, self.colspan, self.rowspan
//        )
//    }
//}

/// Fetches HTML content from a URL or a file.
///
/// # Arguments
///
/// * `source` - A string slice that holds the URL or file path.
/// * `debug` - A boolean to enable debug mode.
///
/// # Returns
///
/// * A Result containing the HTML content as a String if successful, or an error.
async fn fetch_html(source: &str, debug: bool) -> Result<String> {
    let result = if source.starts_with("http://") || source.starts_with("https://") {
        reqwest::get(source)
            .await?
            .text()
            .await
            .context("Failed to fetch URL")
    } else {
        fs::read_to_string(source).context(format!("Failed to read file: {}", source))
    };

    if debug {
        match &result {
            Ok(html) => {
                println!("{}", "Fetched HTML content:".green());
                println!("{}\n", (&html[..html.len().min(200)]).blue()); // Print first 200 characters
            }
            Err(e) => println!("{}", format!("Error fetching HTML: {:?}", e).red()),
        }
    }
    result
}

/// Gets the colspan and rowspan attributes of a cell.
///
/// # Arguments
///
/// * `cell` - An ElementRef representing the cell.
///
/// # Returns
///
/// * A tuple containing the colspan and rowspan values as usize.
fn get_cell_spans(cell: ElementRef) -> (usize, usize) {
    let colspan = cell
        .value()
        .attr("colspan")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let rowspan = cell
        .value()
        .attr("rowspan")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    (colspan, rowspan)
}

/// Extracts tables from the provided HTML content.
///
/// # Arguments
///
/// * `html` - A string slice that holds the HTML content.
/// * `debug` - A boolean to enable debug mode.
///
/// # Returns
///
/// * A Result containing a vector of tables, each table being a vector of rows, and each row being a vector of strings.
fn extract_tables(html: &str, debug: bool) -> Result<Vec<Vec<Vec<String>>>> {
    let document = Html::parse_document(html);
    let table_selector = Selector::parse("table").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td, th").unwrap();

    let mut tables = Vec::new();
    for (table_index, table) in document.select(&table_selector).enumerate() {
        let mut grid: Vec<Vec<Option<Cell>>> = Vec::new();
        let mut max_columns = 0;

        for (row_index, row) in table.select(&row_selector).enumerate() {
            let mut current_row: Vec<Option<Cell>> = Vec::new();
            let mut col_index = 0;

            while col_index < max_columns
                && grid.last().map_or(false, |last_row| {
                    last_row
                        .get(col_index)
                        .map_or(false, |cell| cell.as_ref().map_or(false, |c| c.rowspan > 1))
                })
            {
                if let Some(prev_cell) = &grid.last().unwrap()[col_index] {
                    current_row.push(Some(Cell {
                        content: String::new(),
                        colspan: prev_cell.colspan,
                        rowspan: prev_cell.rowspan - 1,
                    }));
                    col_index += prev_cell.colspan;
                }
            }

            for cell in row.select(&cell_selector) {
                while col_index < current_row.len() && current_row[col_index].is_some() {
                    col_index += 1;
                }
                let (colspan, rowspan) = get_cell_spans(cell);
                let content = cell.text().collect::<String>().trim().to_string();
                let new_cell = Cell {
                    content,
                    colspan,
                    rowspan,
                };

                for _ in 0..colspan {
                    if col_index >= current_row.len() {
                        current_row.push(Some(new_cell.clone()));
                    } else {
                        current_row[col_index] = Some(new_cell.clone());
                    }
                    col_index += 1;
                }
            }
            max_columns = max_columns.max(col_index);

            while current_row.len() < max_columns {
                current_row.push(None);
            }
            if debug {
                let row_content: String = current_row
                    .iter()
                    .map(|cell| match cell {
                        Some(cell) => {
                            format!("['{}', {}, {}]", cell.content, cell.colspan, cell.rowspan)
                        }
                        None => "".to_string(),
                    })
                    .collect::<Vec<_>>().join(", ");
                println!(
                    "{}: {}",
                    format!(
                        "Table {}: Row {}: Columns: {}",
                        table_index + 1,
                        row_index + 1,
                        max_columns
                    )
                    .green(),
                    format!("Cells: {:?}", row_content).blue()
                );
            }
            grid.push(current_row.clone());
        }

        let mut final_table = Vec::new();
        for row in grid {
            let row_data: Vec<String> = row
                .into_iter()
                .map(|cell| cell.map_or(String::new(), |c| c.content))
                .collect();
            final_table.push(row_data);
        }
        if !final_table.is_empty() {
            if debug {
                println!(
                    "{}",
                    format!(
                        "Table {}: Extracted rows: {}",
                        table_index + 1,
                        final_table.len()
                    )
                    .blue()
                );
            }
            tables.push(final_table.clone());
        }
    }

    if debug {
        println!(
            "{}",
            format!("Total tables extracted: {}", tables.len()).blue()
        );
    }
    Ok(tables)
}

/// Saves the extracted tables as CSV files in the specified output directory.
///
/// # Arguments
///
/// * `tables` - A slice of tables, each table being a vector of rows, and each row being a vector of strings.
/// * `output_dir` - A reference to a PathBuf representing the output directory.
///
/// # Returns
///
/// * A Result indicating success or failure.
fn save_tables(tables: &[Vec<Vec<String>>], output_dir: &PathBuf) -> Result<()> {
    fs::create_dir_all(output_dir).context("Failed to create output directory")?;
    for (i, table) in tables.iter().enumerate() {
        let filename = output_dir.join(format!("table_{}.csv", i + 1));
        let mut writer = Writer::from_path(&filename).context("Failed to create CSV file")?;
        for row in table {
            writer.write_record(row).context("Failed to write record")?;
        }
        writer.flush().context("Failed to flush CSV writer")?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let html_content = fetch_html(&cli.input, cli.debug).await?;

    let tables = extract_tables(&html_content, cli.debug)?;
    if tables.is_empty() {
        println!("No tables found in the input source.");
        return Ok(());
    }

    save_tables(&tables, &cli.output_dir)?;
    println!("Successfully extracted {} tables!", tables.len());
    Ok(())
}
