use chrono::Datelike;
use chrono::Local;
use clap::{App, Arg};
use reqwest::{Client, Error as ReqwestError};
use chrono::{DateTime, Utc, TimeZone, FixedOffset};
use serde_json::{json, Value};
use std::fs::File;
use std::io::Write;

const API_URL_SEARCH: &str = "https://etax.exat.co.th/backend/api/search/reprint";
const API_URL_DOWNLOAD: &str = "https://etax.exat.co.th/backend/api/download/zipFiles";
const DATE_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const ONLY_DATE_FORMAT: &str = "%Y%m%d";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let matches = App::new("Tax Document Service")
        .arg(Arg::with_name("taxID").required(true).help("Tax identification number"))
        // Date format: YYYY-MM-DD
        .arg(Arg::with_name("since").short("S").long("since").takes_value(true).help("Start date of the search (default: today)"))
        .arg(Arg::with_name("until").short("U").long("until").takes_value(true).help("End date of the search (default: today)"))
        .arg(Arg::with_name("noDownload").long("no-download").help("Prevent downloading ZIP file"))
        .arg(Arg::with_name("filename").help("Custom filename for the downloaded ZIP (optional)"))
        .get_matches();

    let tax_id = matches.value_of("taxID").unwrap();
    let since_date_str = matches.value_of("since").unwrap_or("");
    let until_date_str = matches.value_of("until").unwrap_or("");
    let no_download = matches.is_present("noDownload");
    let custom_filename = matches.value_of("filename");

    let since_date = parse_date(since_date_str, true)?;
    let until_date = parse_date(until_date_str, false)?;

    let offset = FixedOffset::east(7 * 3600); // GMT+0700
    let doc_date_from = since_date.with_timezone(&offset).format(DATE_FORMAT).to_string();
    let doc_date_to = until_date.with_timezone(&offset).format(DATE_FORMAT).to_string();

    let doc_only_date_from = since_date.with_timezone(&offset).format(ONLY_DATE_FORMAT).to_string();
    let doc_only_date_to = until_date.with_timezone(&offset).format(ONLY_DATE_FORMAT).to_string();

    // Fetch tax document data
    let response_body = fetch_tax_documents(tax_id, &doc_date_from, &doc_date_to).await?;

    // Parse the response to extract necessary data
    let invoice_data = parse_search_response(&response_body)?;

    // Download ZIP file based on flag
    if !no_download {
        download_zip_file(&invoice_data, tax_id, &doc_only_date_from, &doc_only_date_to, custom_filename).await?;
    }

    Ok(())
}

fn parse_date(date_str: &str, start_of_day: bool) -> Result<DateTime<Utc>, chrono::ParseError> {
    let offset = FixedOffset::east_opt(7 * 3600).expect("Invalid timezone offset"); // Use east_opt

    let mut date = Utc::now().date_naive(); // Use Utc::now().date_naive()
    if !date_str.is_empty() {
        date = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
    }

    let datetime = if start_of_day {
        offset.ymd(date.year(), date.month(), date.day()).and_hms(0, 0, 0)
    } else {
        offset.ymd(date.year(), date.month(), date.day()).and_hms(23, 59, 59)
    };

    Ok(datetime.with_timezone(&Utc))
}

async fn fetch_tax_documents(tax_id: &str, doc_date_from: &str, doc_date_to: &str) -> Result<String, ReqwestError> {
    let client = Client::builder().build()?;
    let mut params = std::collections::HashMap::new();
    params.insert("taxId", tax_id);
    params.insert("docDateFrom", doc_date_from);
    params.insert("docDateTo", doc_date_to);
    params.insert("smartCardNo", "null");

    let response = client.post(API_URL_SEARCH)
        .form(&params)
        .send()
        .await?;

    response.text().await
}

fn parse_search_response(response_body: &str) -> Result<String, serde_json::Error> {
    let json_data: Value = serde_json::from_str(response_body)?;
    let data = json_data["reprintList"].as_array().unwrap();

    let listfile: Vec<_> = data.iter().map(|item| {
        println!("docDate: {}, docNo: {}, fileName: {}", item["docDate"], item["docNo"], item["fileName"]);
        json!({
            "invoiceHdr_id": item["invoiceHdrId"],
            "docNo": item["docNo"],
            "fileType": item["fileType"],
            "filePathPDF": item["filePath"],
            "fileNamePDF": item["fileName"],
            "docType": item["docType"]
        })
    }).collect();

    serde_json::to_string(&listfile)
}

async fn download_zip_file(listfile_json: &str, tax_id: &str, doc_date_from: &str, doc_date_to: &str, custom_filename: Option<&str>) -> Result<(), Box<dyn std::error::Error>> { // Change return type
    let client = Client::builder().build()?;

    let form = reqwest::multipart::Form::new()
        .text("listfile", listfile_json.to_string())
        .text("type", "PDF");

    let response = client.post(API_URL_DOWNLOAD)
        .multipart(form)
        .send()
        .await?;

    let content = response.bytes().await?;

    let filename = match custom_filename {
        Some(name) => name.to_string(),
        None => {
            let now = Local::now();
            format!("TaxDocuments_{}_{}_{}_{}.zip", tax_id, doc_date_from, doc_date_to, now.format("%Y%m%d%H%M%S"))
        }
    };

    let mut file = File::create(&filename).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;
    file.write_all(&content).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    println!("Zip file downloaded successfully.");

    Ok(())
}
