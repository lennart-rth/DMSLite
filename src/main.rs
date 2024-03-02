use tesseract_sys::*;
use std::process::Command;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::Ollama;
use std::fs;

use leptonica_sys::{pixFreeData, pixRead};
use std::ffi::CStr;
use std::ptr;


#[tokio::main]
async fn main() {
    let ocr_string;
    let pdf2jpg = Command::new("pdftoppm")
        .arg("-jpeg")
        .arg("test.pdf")
        .arg("test")
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

    unsafe {
    let cube = TessBaseAPICreate();
    TessBaseAPIInit3(cube, ptr::null(), b"eng\0".as_ptr().cast());
    let image = pixRead(b"consume/test-1.jpg\0".as_ptr().cast());
    TessBaseAPISetImage2(cube, image);
    TessBaseAPIRecognize(cube, ptr::null_mut());
    let text = TessBaseAPIGetUTF8Text(cube);
    let ocr_result = CStr::from_ptr(text).to_str();

    if ocr_result.is_ok() == true {
        ocr_string = ocr_result.unwrap();
    } else{
        eprintln!("Error while OCR.");
        ocr_string = "";
    }

    TessDeleteText(text);
    pixFreeData(image);
    TessBaseAPIDelete(cube);
    }
    // println!("{}",ocr_string);

    let _ = fs::remove_file("consume/test-1.jpg");


    llm_inference(ocr_string).await;
}

async fn llm_inference(ocr: &str) {

    let new_ocr = ocr.chars()
    // note: there's an edge case if ch == char::MAX which we must decide
    //       how to handle. in this case I chose to not change the
    //       character, but this may be different from what you need.
    .map(|ch| {
        if ch.is_ascii() {
            char::from_u32(ch as u32 + 1).unwrap_or(ch)
        } else {
            ch
        }
    })
    .collect::<String>();
    let ollama = Ollama::default();
    let model = "gemma:2b".to_string();
    let prompt = new_ocr;
    
    let res = ollama.generate(GenerationRequest::new(model, prompt)).await;
    
    if let Ok(res) = res {
        println!("{}", res.response);
    }
}