use chrono::NaiveDate;

// Postgres main_table equivalent for Rust
pub struct Document {
    pub _id: i32,
    pub upload_date: NaiveDate,
    pub filepath: String,
    pub title: String
}

// Postgres Document_content equivalent for Rust
pub struct DocumentContent {
    pub _id: i32,
    pub content: String,
    pub summary: String,
    pub buzzwords: String
}

// Postgres Search result equivalent for Rust
#[derive(Debug)]
pub struct SearchResult {
    pub _id: i32,
    pub title: String,
    pub upload_date: NaiveDate,
    pub rank: f32
}


// Settings for the Postgres DB and the Dirs
pub mod settings {
    pub const CONSUME_PATH: &str = "/home/lennart/DMSLite/consume/";
    pub const STORAGE_PATH: &str = "/home/lennart/DMSLite/storage/";
    pub const PSQL_HOST: &str = "localhost";
    pub const PSQL_USER: &str = "dmslite";
    pub const PSQL_PASSWD: &str = "dmslite";
    pub const PSQL_DBNAME: &str = "dmslite";
    pub const TESSERACT_LANG: &str = "deu";
}