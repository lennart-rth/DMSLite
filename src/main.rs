use std::process::Command;
use std::fs;
use chrono::{NaiveDate, Utc};
use ids_service::crypto_hash::*;
use std::path::{Path, PathBuf};
use std::io;

mod settings;
mod ocr;
mod llm;
mod psql;
use crate::psql::Database;

//Takes a Path and a filename and returns the the path to the changed File with its new name. 
fn change_file_name(path: impl AsRef<Path>, name: &str) -> PathBuf {
    let path = path.as_ref();
    let mut result = path.to_owned();
    result.set_file_name(name);
    if let Some(ext) = path.extension() {
        result.set_extension(ext);
    }
    result
}

// Creates a Entry for the Postgres DB.
// 1. Use OCR to get the content of the Doc. -> Saved in output.txt inside the consume folder.
// 2. Use Ollama to generate summaries and classify the Doc based on its content.
// 3. Generate the structs Document and Document_content
// 4. Copy the document from the Consume folder into the long-time storage folder with a unique hash as the filename
async fn create_entry(name: String) -> (settings::Document, settings::DocumentContent) {
    // Read Content
    ocr::ocr(name.clone());

    // Generate Texts
    let (content, summary, buzzwords, title) = llm::llm_prompt().await;

    // Copy File into Storage Dir with Hash as the name.
    let hash = create_id_as_sha256();
    let old_path = settings::settings::CONSUME_PATH;
    let new_name = change_file_name(&name.clone(), &hash).into_os_string().into_string().unwrap();
    let new_path = settings::settings::STORAGE_PATH.to_owned()+&new_name;
    let _ = match fs::rename(old_path.to_owned()+&name, new_path.clone()) {
        Ok(()) => (),
        Err(e) => {
            eprint!("Error: {}", e);
        },
    };

    // Define PSQL Structs
    let upload_date = Utc::now().date_naive();
    let document = settings::Document {
        _id: 0,
        upload_date: upload_date,
        filepath: new_path.clone(),
        title: title
    };

    let document_content = settings::DocumentContent {
        _id: 0,
        content: content,
        summary: summary,
        buzzwords: buzzwords
    };

    // Clean up
    let del = clean_up();
    match del {
        Ok(()) => (),
        Err(e) => eprintln!("Cant clean up the consume dir: {}", e),
    }
    
    (document, document_content)
}

// OCR and Ollama leave Files in the Consume dir.
// clean_up deletes them and any other accidentally generated ".jpg" files.
fn clean_up() -> io::Result<()> {
    let _ = fs::remove_file(settings::settings::CONSUME_PATH.to_owned()+"output.txt");

    let entries = fs::read_dir(settings::settings::CONSUME_PATH)?;
    for entry in entries {
        let entry = entry?;
        let file_path = entry.path();

        if let Some(extension) = file_path.extension() {
            if extension == "jpg" {
                // Delete the file
                fs::remove_file(&file_path)?;
                println!("Deleted file: {:?}", file_path);
            }
        }
    }
    Ok(())
}

// Consume all Files in the Consume dir.
// 1. Find all Files in the Dir
// 2. if File is PDF, create the Entry for each file
// 3. Uplaod the File to the Postgres DB
async fn consume() {
    let paths = fs::read_dir(settings::settings::CONSUME_PATH).unwrap();
    let names = paths.filter_map(|entry| {
        entry.ok().and_then(|e|
            e.path().file_name()
            .and_then(|n| n.to_str().map(|s| String::from(s)))
        )
        }).collect::<Vec<String>>();

    for name in names.clone() {
        if name.ends_with(".pdf") {
            println!("Consuming: {}", &name);
            let (document, document_content) = create_entry(name).await;
            match psql::add_to_psql(document, document_content).await {
                Ok(_) => 
                    println!("Database succesfully updated."),
                Err(e) => 
                    eprintln!("Error updateing Database: {}",e)
            }
        }
    }

    if names.len() == 0 {
        println!("Nothing to consume!");
    }
}


// Infinite loop to take in commands.
// Call functions to execute the commands.
// check DB row count and show after command returns
#[tokio::main]
async fn main() {
    loop {
        println!("Please enter a command (_c_onsume || _s_earch <term> || _o_pen <id> || _d_elete <id>  || _l_ist all || _q_uit):");
        let mut input = String::new();
        io::stdin().read_line(&mut input).expect("Failed to read line");

        let mut words = input.trim().split_whitespace();
        let cmd = words.next().unwrap_or("");
        let parameter = words.last().unwrap_or("");


        match cmd {
            "c" => consume().await,
            "s" => render_search(parameter.to_string()).await,
            "d" => delete(parameter.to_string()).await,
            "o" => open_file(parameter.to_string()).await,
            "l" => list_all().await,
            "q" => {
                break;
            }
            _ => println!("Invalid command!"),
        }

        let db: Database = psql::Database::init().await.unwrap();
        tokio::spawn(async move {
            if let Err(e) = db.connection.expect("Coudlnt find Connection to Psql").await {
                eprintln!("Psql connection error: {}", e);
            }
        });
        let rows: i64 = match db.client.expect("Databse init failed!").query("SELECT COUNT(*) FROM main_table;", &[]).await {
            Ok(row) => row[0].get(0),
            Err(e) => {
                eprintln!("Postgres row count failed with: {}", e);
                return;
            }
        };

        println!("{rows} Documents stored!");
        
        
    }
}

// List all Documents in the Database
async fn list_all() {
    let db = psql::Database::init().await.unwrap();
    tokio::spawn(async move {
        if let Err(e) = db.connection.expect("Coudlnt find Connection to Psql").await {
            eprintln!("Psql connection error: {}", e);
        }
    });
    let client = db.client.expect("Psql Cient not found");
    // Prepare and execute the search query
    let all = match client.query("SELECT id, title, upload_date FROM main_table;",&[]).await {
        Ok(row) => row,
        Err(e) => {
            eprintln!("Postgres list all error: {}", e);
            return;
        }
    };

    if all.len() != 0 {
        println!("+========+==============================================+==============+");
        println!("|   ID   |    TITLE                                     |     DATE     |");
        println!("+========+==============================================+==============+");
    } else {
        println!("No Results");
    }

    for row in all {
        let mut title:String = row.get(1);
        if title.len() > 46 {
            title = title.replace("\n", "");
            title.truncate(46);
        }
        
        let id: i32 = row.get(0);
        let date: NaiveDate = row.get(2);
        println!("{}",format!("|{: ^8}|{: ^46}|{: ^14}|", id, title, date.to_string()));
        println!("+--------+----------------------------------------------+--------------+");
    }
}

// Delete a Docuemnt by its Id
// Delete the Document from the long-time storage folder.
async fn delete(id_s: String) {
    let id: i32 = id_s.parse().unwrap_or(-1);
    if id > 0 {
        let db = psql::Database::init().await.unwrap();
        tokio::spawn(async move {
            if let Err(e) = db.connection.expect("Coudlnt find Connection to Psql").await {
                eprintln!("psql connection error: {}", e);
            }
        });
        let mut client = db.client.expect("Psql Cient not found");

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

// Call the search and list the results formatted in the terminal.
async fn render_search(parameter: String) {

    let mut results = Vec::new();
    match psql::search(parameter.trim().to_string()).await {
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
        let mut title:String = md.title;
        if title.len() > 46 {
            title = title.replace("\n", "");
            title.truncate(46);
        }
    
        println!("{}",format!("|{: ^8}|{: ^46}|{: ^12}|{: ^14}|", md._id, title, md.rank, md.upload_date.to_string()));
        println!("+--------+----------------------------------------------+------------+--------------+");

    }
}

// Open the file with teh <id> with its standart programm using xdg-open. 
async fn open_file(id_s :String) {
    let id: i32 = id_s.parse().unwrap_or(-1);
    if id > 0 {
        let db = psql::Database::init().await.unwrap();
        tokio::spawn(async move {
            if let Err(e) = db.connection.expect("Coudlnt find Connection to Psql").await {
                eprintln!("psql connection error: {}", e);
            }
        });
        let client = db.client.expect("Psql Cient not found");

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


        let output = Command::new("xdg-open")
        .arg(filepath)
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
