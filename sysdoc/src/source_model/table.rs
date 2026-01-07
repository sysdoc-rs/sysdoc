//! CSV table source types

use std::path::PathBuf;

/// A CSV table source file
#[derive(Debug, Clone)]
pub struct TableSource {
    /// Path to the CSV file (relative to document root)
    pub path: PathBuf,

    /// Absolute path to the CSV file
    pub absolute_path: PathBuf,

    /// Whether the table has been loaded into memory
    pub loaded: bool,

    /// Parsed CSV data (if loaded)
    pub data: Option<Vec<Vec<String>>>,
}

impl TableSource {
    /// Load and parse the CSV data
    ///
    /// # Returns
    /// * `Ok(())` - Successfully loaded and parsed CSV data into memory
    /// * `Err(csv::Error)` - Error reading or parsing the CSV file
    pub fn load(&mut self) -> Result<(), csv::Error> {
        let mut reader = csv::Reader::from_path(&self.absolute_path)?;
        let mut data = Vec::new();

        // Read the headers as the first row
        let headers = reader.headers()?;
        let header_row: Vec<String> = headers.iter().map(|s| s.to_string()).collect();
        data.push(header_row);

        // Read the data rows
        for result in reader.records() {
            let record = result?;
            let row: Vec<String> = record.iter().map(|s| s.to_string()).collect();
            data.push(row);
        }

        self.data = Some(data);
        self.loaded = true;
        Ok(())
    }
}
