use clap::Parser;
use csv::Writer;
use reqwest;
use scraper::{Html, Selector, ElementRef};
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};

#[derive(Parser)]
#[command(author, version, about = "Extract tables from HTML files and save them as CSV", long_about = None)]
struct Cli {
    /// Input HTML file path or URL
    #[arg(short, long)]
    input: String,

    /// Output directory for CSV files
    #[arg(short, long, default_value = ".")]
    output_dir: PathBuf,
}

#[derive(Clone,Debug)]
struct Cell {
    content: String,
    colspan: usize,
    rowspan: usize,
}

async fn fetch_html(source: &str) -> Result<String> {
    if source.starts_with("http://") || source.starts_with("https://") {
        Ok(reqwest::get(source)
            .await?
            .text()
            .await?)
    } else {
        Ok(fs::read_to_string(source)
            .with_context(|| format!("Failed to read file: {}", source))?)
    }
}

fn get_cell_spans(cell: ElementRef) -> (usize, usize) {
    let colspan = cell.value().attr("colspan")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let rowspan = cell.value().attr("rowspan")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    (colspan, rowspan)
}

fn extract_tables(html: &str) -> Result<Vec<Vec<Vec<String>>>> {
    let document = Html::parse_document(html);
    let table_selector = Selector::parse("table").unwrap();
    let row_selector = Selector::parse("tr").unwrap();
    let cell_selector = Selector::parse("td, th").unwrap();

    let mut tables = Vec::new();

    for table in document.select(&table_selector) {
        let mut grid: Vec<Vec<Option<Cell>>> = Vec::new();
        let mut max_columns = 0;

        // First pass: collect all rows and determine the table dimensions
        for row in table.select(&row_selector) {
            let mut current_row: Vec<Option<Cell>> = Vec::new();
            let mut col_index = 0;

            // Fill in any cells from previous rows' rowspans
            while col_index < max_columns && grid.last().map_or(false, |last_row| {
                last_row.get(col_index).map_or(false, |cell| {
                    cell.as_ref().map_or(false, |c| c.rowspan > 1)
                })
            }) {
                if let Some(prev_cell) = &grid.last().unwrap()[col_index] {
                    current_row.push(Some(Cell {
                        content: prev_cell.content.clone(),
                        colspan: prev_cell.colspan,
                        rowspan: prev_cell.rowspan - 1,
                    }));
                    col_index += prev_cell.colspan;
                }
            }

            // Process cells in the current row
            for cell in row.select(&cell_selector) {
                // Skip columns that are already filled by a previous colspan
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

                // Fill in all columns this cell spans
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
            
            // Pad the row to max_columns with None
            while current_row.len() < max_columns {
                current_row.push(None);
            }

            grid.push(current_row);
        }

        // Convert grid to final table format
        let mut final_table = Vec::new();
        for row in grid {
            let row_data: Vec<String> = row.into_iter()
                .map(|cell| cell.map_or(String::new(), |c| c.content))
                .collect();
            final_table.push(row_data);
        }

        if !final_table.is_empty() {
            tables.push(final_table);
        }
    }

    Ok(tables)
}

fn save_tables(tables: &[Vec<Vec<String>>], output_dir: &PathBuf) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    for (i, table) in tables.iter().enumerate() {
        let filename = output_dir.join(format!("table_{}.csv", i + 1));
        let mut writer = Writer::from_path(&filename)?;

        for row in table {
            writer.write_record(row)?;
        }
        writer.flush()?;
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Fetch HTML content
    let html_content = fetch_html(&cli.input).await?;

    // Extract tables
    let tables = extract_tables(&html_content)?;

    if tables.is_empty() {
        println!("No tables found in the input source.");
        return Ok(());
    }

    // Save tables as CSV files
    save_tables(&tables, &cli.output_dir)?;
    println!("Successfully extracted {} tables!", tables.len());

    Ok(())
}
