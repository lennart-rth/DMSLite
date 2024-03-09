use std::process::Command;

use crate::settings::settings;


// To use Tesseract for Ocr this function converts the PDF into a JPG.
pub fn pdf2jpg(name: String) {
    let pdf2jpg = Command::new("pdftoppm")
        .arg("-jpeg")
        .arg(name.clone())
        .arg(name)
        .stdout(std::process::Stdio::null())
        .current_dir(settings::CONSUME_PATH)
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

// Convert PDF to Image. Image is named "<pdf_name>-1.jpg"
// Get text in the Image and save it in a "output.txt" file
pub fn ocr(name: String) {
    pdf2jpg(name.clone());

    let ppm_out_name = name.split(".").next().unwrap_or_default();

    let tesseract = Command::new("tesseract")
    .arg(ppm_out_name.to_owned()+".pdf-1.jpg")
    .arg("output")
    .arg("-l")
    .arg(settings::TESSERACT_LANG)
    .stdout(std::process::Stdio::null())
    .current_dir(settings::CONSUME_PATH)
    .status()
    .expect("failed to execute process");

    match tesseract.code() {
        Some(0) => println!("Success using tessercat as ocr"),
        Some(code) => eprintln!("Error using tesseract. code: {}",code),
        None => eprintln!("Process terminated by signal")
    }
}