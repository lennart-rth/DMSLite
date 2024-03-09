use tokio_postgres::{config::Config, NoTls, Error, Client, Connection, Socket};
use tokio_postgres::tls::NoTlsStream;

use crate::settings::settings;
use crate::settings::SearchResult;
use crate::settings::Document;
use crate::settings::DocumentContent;

pub struct Database {
    pub client: Option<Client>,
    pub connection:Option<Connection<Socket, NoTlsStream>>,
}

impl Database {
    pub async fn init() -> Result<Database, tokio_postgres::Error> {
        let mut config = Config::new();
        config.host(settings::PSQL_HOST);
        config.user(settings::PSQL_USER);
        config.password(settings::PSQL_PASSWD);
        config.dbname(settings::PSQL_DBNAME);

        let (client, connection) = config.connect(NoTls).await?;
        
        Ok(Self {
            client: Some(client),
            connection: Some(connection),
        })
    }

}

// Add the content of the Document and Document_content Struct in the DB.
pub async fn add_to_psql(document: Document, document_content: DocumentContent) -> Result<(), Error> {
    let db: Database = Database::init().await.unwrap();
    // Begin a transaction
    let mut client = db.client.expect("Psql Cient not found");
    let transaction = client.transaction().await?;
    // Insert data into main_table
    transaction.execute(
        "INSERT INTO dmslite.main_table (upload_date, filepath, title) VALUES ($1, $2, $3)",
        &[&document.upload_date, &document.filepath, &document.title],
    ).await?;

    transaction.execute(
        "WITH inserted_id AS (
            INSERT INTO dmslite.document_content (id, content, summary, buzzwords)
            SELECT currval('dmslite.main_table_id_seq'), $1, $2, $3
            RETURNING id
        )
        SELECT id FROM inserted_id",
        &[&document_content.content, &document_content.summary, &document_content.buzzwords],
    ).await?;

    // Commit the transaction
    transaction.commit().await?;

    println!("Data inserted successfully");

    Ok(())

}

// fuzzy search for a Phrase in the Columns content, summary and buzzwords,
// order them by word_similarity distnce and return all values over sensitivity threshold.
pub async fn search(search_term: String) -> Result<Vec<SearchResult>, Error> {
    let sensitivity: f32 = 0.6;
    let mut results: Vec<SearchResult> = Vec::new();

    let db: Database = Database::init().await.unwrap();
    let client = db.client.expect("Psql Cient not found");

    // Prepare and execute the search query
    for row in client.query("SELECT DISTINCT main_table.id, subquery.distance, main_table.title, main_table.upload_date
    FROM (
        SELECT id, $1 <<-> content AS distance
        FROM document_content
        WHERE $1 <<-> content < $2 OR $1 <<-> content = 0
        UNION
        SELECT id, $1 <<-> summary AS distance
        FROM document_content
        WHERE $1 <<-> summary < $2 OR $1 <<-> summary = 0
        UNION
        SELECT id, $1 <<-> buzzwords AS distance
        FROM document_content
        WHERE $1 <<-> buzzwords < $2 OR $1 <<-> buzzwords = 0
    ) AS subquery
    JOIN main_table ON subquery.id = main_table.id
    ORDER BY subquery.distance ASC;",
        &[&search_term, &sensitivity],
    ).await? {
        let search_r = SearchResult { _id: row.get(0), rank:row.get(1), title:row.get(2), upload_date:row.get(3)};
        results.push(search_r);
    }
    Ok(results)
}
