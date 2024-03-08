use std::process::Command;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use std::fs;
use tokio_postgres::{config::Config, NoTls, Error};
use chrono::{NaiveDate, Utc};
use ids_service::crypto_hash::*;
use std::path::{Path, PathBuf};
use std::io;


struct Document {
    _id: i32,
    upload_date: NaiveDate,
    filepath: String,
    title: String
}

struct DocumentContent {
    _id: i32,
    content: String,
    summary: String,
    buzzwords: String
}

#[derive(Debug)]
struct SearchResult {
    _id: i32,
    title: String,
    upload_date: NaiveDate,
    rank: f32
}


fn pdf2jpg(name: String) {
    let pdf2jpg = Command::new("pdftoppm")
        .arg("-jpeg")
        .arg(name)
        .arg("image")
        .stdout(std::process::Stdio::null())
        .current_dir("/home/lennart/Documents/Repos/dmslite/consume/")
        .status()
        .expect("failed to execute process");

    match pdf2jpg.code() {
        Some(0) => println!("Success converting pdf to jpg"),
        Some(1) => eprintln!("Error converting pdf to jpg:\nError opening PDF file."),
        Some(2) => eprintln!("Error converting pdf to jpg:\nError opening an output file."),
        Some(3) => eprintln!("Error converting pdf to jpg:\nError related to PDF permissions."),
        Some(99) => eprintln!("Error converting pdf to jpg:\nOther error."),
        Some(code) => eprintln!("Error converting pdf to jpg:\nOther error. code: {}",code),
        None => eprintln!("Process terminated by signal")
    }

}

fn ocr(name: String) {
    pdf2jpg(name);

    let tesseract = Command::new("tesseract")
    .arg("image-1.jpg")
    .arg("output")
    .arg("-l")
    .arg("deu")
    .stdout(std::process::Stdio::null())
    .current_dir("/home/lennart/Documents/Repos/dmslite/consume/")
    .status()
    .expect("failed to execute process");

    match tesseract.code() {
        Some(0) => println!("Success using tessercat as ocr"),
        Some(code) => eprintln!("Error using tesseract. code: {}",code),
        None => eprintln!("Process terminated by signal")
    }
}

fn tidy_up_string(mut string: String) -> String {
    let mut prev = ' ';
    string = string.trim().to_owned();
    string.retain(|ch| {
        let result = ch != ' ' || prev != ' ';
        prev = ch;
        result
    });
    string
}

async fn llm_prompt() -> (String, String, String, String) {
    let mut contents = fs::read_to_string("consume/output.txt".to_string())
        .expect("Should have been able to read the file");
    contents.retain(|c| c.is_ascii());
    contents.retain(|c| !c.is_ascii_control());
    contents = tidy_up_string(contents);
    // println!("{}",contents);


    let mut summary = llm_inference(contents.clone(), "doc_summarizer".to_string()).await;
    let mut buzzwords = llm_inference(contents.clone(), "doc_buzword_generator".to_string()).await;
    let mut title = llm_inference(buzzwords.clone(), "doc_title_generator".to_string()).await;

    summary = tidy_up_string(summary);
    buzzwords = tidy_up_string(buzzwords);
    title = tidy_up_string(title);
    title = title.replace("*", "");

    (contents, summary, buzzwords, title)
}

async fn llm_inference(ocr: String, model: String) -> String{
    let ollama = Ollama::default();
    let model = model;
    let prompt = ocr;
    
    let res = ollama.generate(GenerationRequest::new(model, prompt)).await;
    
    if let Ok(res) = res {
        return res.response;
    } else {
        return "".to_string();
    }
}

fn change_file_name(path: impl AsRef<Path>, name: &str) -> PathBuf {
    let path = path.as_ref();
    let mut result = path.to_owned();
    result.set_file_name(name);
    if let Some(ext) = path.extension() {
        result.set_extension(ext);
    }
    result
}


async fn create_entry(name: String) -> (Document, DocumentContent) {
    // Read Content
    ocr(name.clone());

    // Generate Texts
    let (content, summary, buzzwords, title) = llm_prompt().await;

    // Copy File into Storage Dir with Hash as the name.
    let hash = create_id_as_sha256();
    let old_path = "consume/".to_string();
    let new_name = change_file_name(&name.clone(), &hash).into_os_string().into_string().unwrap();
    let new_path = "/home/lennart/DMSLITE/".to_owned()+&new_name;
    let _ = fs::rename(old_path+&name, new_path.clone());

    // Define PSQL Structs
    let upload_date = Utc::now().date_naive();
    let document = Document {
        _id: 0,
        upload_date: upload_date,
        filepath: new_path.clone(),
        title: title
    };

    let document_content = DocumentContent {
        _id: 0,
        content: content,
        summary: summary,
        buzzwords: buzzwords
    };

    // Clean up
    let _ = fs::remove_file("consume/image-1.jpg");
    let _ = fs::remove_file("consume/output.txt");
    
    (document, document_content)
}

async fn add_to_psql(document: Document, document_content: DocumentContent) -> Result<(), Error> {
    let mut config = Config::new();
    config.host("localhost");
    config.user("dmslite");
    config.password("dmslite"); // Replace "your_password" with your actual password
    config.dbname("dmslite");

    let (mut client, connection) = config.connect(NoTls).await?;
    // Spawn a task to process the connection in the background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });
    // Begin a transaction
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

async fn consume() {
    let paths = fs::read_dir("./consume").unwrap();
    let names = paths.filter_map(|entry| {
        entry.ok().and_then(|e|
            e.path().file_name()
            .and_then(|n| n.to_str().map(|s| String::from(s)))
        )
        }).collect::<Vec<String>>();

    for name in names.clone() {
        println!("Consuming: {}", &name);
        let (document, document_content) = create_entry(name).await;
        match add_to_psql(document, document_content).await {
            Ok(_) => 
                println!("Database succesfully updated."),
            Err(e) => 
                eprintln!("Error updateing Database: {}",e)
        }
    }

    if names.len() == 0 {
        println!("Noting to consume!");
    }
}

async fn search(search_term: String) -> Result<Vec<SearchResult>, Error> {
    let sensitivity: f32 = 0.6;
    let mut results: Vec<SearchResult> = Vec::new();


    let mut config = Config::new();
    config.host("localhost");
    config.user("dmslite");
    config.password("dmslite"); // Replace "your_password" with your actual password
    config.dbname("dmslite");

    // Establish connection to the PostgreSQL database
    let (client, connection) = config.connect(NoTls).await?;
    // Spawn a task to process the connection in the background
    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

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




#[tokio::main]
async fn main() {
    loop {
        println!("Please enter a command (c-onsume/s-earch <term>/p-arse/o-pen <id>/d-elete <id>/q-uit):");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");

        let mut words = input.trim().split_whitespace();
        let cmd = words.next().unwrap_or("");
        let parameter = words.last().unwrap_or("");

        match cmd {
            "c" => consume().await,
            "s" => render_search(parameter.to_string()).await,
            "p" => parse(),
            "d" => delete(parameter.to_string()).await,
            "o" => open_file(parameter.to_string()),
            "q" => {
                break;
            }
            _ => println!("Invalid command!"),
        }
    }
}


async fn delete(id_s: String) {
    let id: i32 = id_s.parse().unwrap_or(-1);
    if id > 0 {
        let mut config = Config::new();
        config.host("localhost");
        config.user("dmslite");
        config.password("dmslite"); // Replace "your_password" with your actual password
        config.dbname("dmslite");

        let (mut client, connection) = match config.connect(NoTls).await {
            Ok(value) => value,
            Err(e) => {
                eprintln!("Postgres connection error: {}", e);
                return;
            }
        };


        // Spawn a task to process the connection in the background
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        let filepath_rows = match client.query("SELECT filepath FROM main_table WHERE id = $1;", &[&id]).await {
            Ok(row) => row,
            Err(e) => {
                eprintln!("Postgres SELECT Error: Cant get filepath: {}", e);
                return;
            }
        };

        let mut filepath: String = "".to_string();
        // We expect only one row
        if let Some(row) = filepath_rows.get(0) {
            if let Some(filepath_value) = row.try_get::<_, String>(0).ok() {
                filepath = filepath_value;
            } else {
                eprintln!("Error: Couldn't extract filepath from row.");
            }
        } else {
            eprintln!("Error: No rows returned from the query.");
        }

        let transaction = match client.transaction().await {
            Ok(value) => value,
            Err(e) => {
                eprintln!("Transaction error: {}", e);
                return;
            }
        };

        if let Err(e) = transaction.execute(
            "DELETE FROM main_table
            WHERE id = $1;",
            &[&id],
        ).await {
            eprintln!("Postgres delete error: {}", e);
            return;
        };

        // Commit the transaction
        if let Err(e) = transaction.commit().await {
            eprintln!("Posgres transaction commit error: {}", e);
            return;
        };
        
        // Attempt to remove the file
        println!("{}", filepath);
        match fs::remove_file(filepath) {
            Ok(()) => println!("File deleted successfully"),
            Err(err) => eprintln!("Error deleting file: {}", err),
        }
        
    } 
}


async fn render_search(parameter: String) {

    let mut results = Vec::new();
    match search(parameter.trim().to_string()).await {
        Ok(r) => results = r,
        Err(e) => eprintln!("Error: {}", e),
    }

    if results.len() != 0 {
        println!("+========+==============================================+============+==============+");
        println!("|   ID   |    TITLE                                     |    RANK    |     DATE     |");
        println!("+========+==============================================+============+==============+");
    } else {
        println!("No Results");
    }

    for md in results {
        println!("{}",format!("|{: ^8}|{: ^46}|{: ^12}|{: ^14}|", md._id, md.title, md.rank, md.upload_date.to_string()));
        println!("+--------+----------------------------------------------+------------+--------------+");

    }
}


fn parse() {
    println!("You called command parse.");
}

fn open_file(id_s :String) {
    let id: i32 = id_s.parse().unwrap_or(-1);
    if id > 0 {
        let filename = "doc_summarizer";
        let output = Command::new("sudo")
        .arg("xdg-open")
        .arg(filename)
        .output()
        .expect("failed to execute process");

        // Check if there's any error while executing the command
        if !output.status.success() {
            eprintln!(
                "Error executing xdg-open: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
}