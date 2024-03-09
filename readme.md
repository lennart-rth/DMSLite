![Crates.io MSRV (version)](https://img.shields.io/crates/msrv/dmslite/:version)
# DMSLite
DMS Lite is a secure and lightweight command-line tool for document management. It provides efficient document indexing, searching, and AI-based categorization, ensuring fast performance even with large document volumes, all while keeping operations entirely local on your machine for maximum privacy and security.

## Usage
If your roots bin folder is in $PATH you can type `dmslite` everywhere to:
1. __Consume Documents:__ Add documents to a specified folder to process. (E.g. with the command `c`)
2. __Search Documents:__ Use the CLI to search for documents by content, title, or creation date (fuzzy word similarity search). (E.g. with the command `s` followed by the search phrase)
3. __Open Documents:__ Open a Document found with the search with its default application right from the cli tool. (E.g. with the command `o` followed by the id found out by a search before)
4. __Delete Documents:__ Delete a Document found with the search by its id. (E.g. with the command `d` followed by the id found out by a prior search.)

## Installation and Setup

### `cargo install dmslite`

### Prerequisites
1. [PostgreSQL](https://www.postgresql.org/) database
2. [Tesseract](https://github.com/tesseract-ocr/tesseract) installed in your local language
3. [Ollama](https://ollama.com/) setup with a local Model.
4. pdftoppm (Installed with `sudo apt install poppler-utils`)
5. xdg-open. To be able to open Docuemnts right from the terminal.

### PostgreSQL Database Setup
1. CREATE USER dmslite WITH PASSWORD 'dmslite';
2. As a __psql superuser__, create a PostgreSQL databaseand Schema:
```
    psql -U postgres
    CREATE DATABASE dmslite OWNER dmslite;
    CREATE SCHEMA dmslite;
```
3. Write your Password under `src/settings.rs` in the String `PSQL_PASSWD`
4. As the dmslite User, create search Indices, main_table and document_content table
    ```
    CREATE EXTENSION pg_trgm;

    -- create indices
    CREATE INDEX idx_content_trgm ON document_content USING gin (content gin_trgm_ops);
    CREATE INDEX idx_summary_trgm ON document_content USING gin (summary    gin_trgm_ops);
    CREATE INDEX idx_buzzwords_trgm ON document_content USING gin (buzzwords gin_trgm_ops);

    -- create tables
    CREATE TABLE dmslite.main_table (
        id SERIAL PRIMARY KEY,
        upload_date DATE,
        filepath VARCHAR(255),
        title TEXT
    );

    CREATE TABLE dmslite.document_content (
        id SERIAL PRIMARY KEY,
        -- Other columns in table2
        content TEXT,
        summary TEXT,
        buzzwords TEXT,
        -- Add more columns as needed
        FOREIGN KEY (id) REFERENCES main_table(id) ON DELETE CASCADE
    );
    ```

### Ollama Custom Models Setup

    Build custom Ollama models:
    ```
    ollama create doc_buzzword_generator -f doc_buzzword_generator
    ollama create doc_summarizer -f doc_summarizer
    ollama create doc_title_generator -f doc_title_generator
    ```

### Settings
1. Make a folder for consumation of documents.
2. Make a folder for indexed storage of documents.
3. Write the two absolute folder paths to the strings `CONSUME_PATH` and `STORAGE_PATH` in the file `src/settings.rs`. \
 __They must be <u>absolute</u> paths starting with `/home/<user>/...`__
4. Set the String `TESSERACT_LANG` to your tesseract Language flag. (E.g. "eng" or "deu")

## Uninstall/Delete

### Postgres

```
DROP TABLE document_content;
DROP TABLE main_table;
DROP INDEX IF EXISTS idx_content_trgm;
DROP INDEX IF EXISTS idx_summary_trgm;
DROP INDEX IF EXISTS idx_buzzwords_trgm;
DROP FUNCTION fuzzy_search_document_content;
```

### Ollama
```
ollama rm doc_buzzword_generator
ollama rm doc_summarizer
ollama rm doc_title_generator
```