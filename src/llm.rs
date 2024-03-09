use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use std::fs;

use crate::settings::settings;

// Remove chain of more then one whitespace char to only one.
// E.g. "text  \n   text" to "text text"
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

//  prompt all LLMs over the document context that is saved in the generated output.txt file from Tesseract.
pub async fn llm_prompt() -> (String, String, String, String) {
    let mut contents = fs::read_to_string(settings::CONSUME_PATH.to_owned()+"output.txt")
        .expect("Should have been able to read the file");
    contents.retain(|c| c.is_ascii());
    contents.retain(|c| !c.is_ascii_control());
    contents = tidy_up_string(contents);
    // println!("{}",contents);


    let mut summary = llm_inference(contents.clone(), "doc_summarizer".to_string()).await;
    let mut buzzwords = llm_inference(contents.clone(), "doc_buzzword_generator".to_string()).await;
    let mut title = llm_inference(buzzwords.clone(), "doc_title_generator".to_string()).await;

    summary = tidy_up_string(summary);
    buzzwords = tidy_up_string(buzzwords);
    title = tidy_up_string(title);
    title = title.replace("*", "");

    (contents, summary, buzzwords, title)
}

// Generate Answer for a LLM with User Input (ocr).
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