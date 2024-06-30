use rss::{ChannelBuilder, ItemBuilder};
use std::{fs, env, process::Command};
use crate::config::toml::{Conf, Object, Main};

const HELP: &str = r#"
Adduce Feed - create blogs or other simple documents.

Usage: adduce feed [COMMAND] <argument>

Commands:
    establish               create feed structure
    create <file_name>      create new document
    remove <file_name>      delete a document
    edit <file_name>        modify an existing document
    export <file_name>      generate HTML from document
    search <query>          search your documents
    rss                     generate RSS feed

See `adduce` for Adduce Standard usage.
"#;

pub fn process(args: Vec<String>) {

    if args.len() < 2 {
        println!("{HELP}");
        return;
    }

    let command = args[1].as_str();

    match command {
        "establish" => cli_establish(),
        "rss" => cli_rss(),
        "create" | "remove" | "edit" | "export" | "search" => {
            if args.len() < 3 {
                println!("{HELP}");
                return;
            }
            let argument = args[2].as_str();
            match command {
                "create" => cli_create(argument),
                "remove" => cli_remove(argument),
                "edit" => cli_edit(argument),
                "export" => cli_export(argument),
                "search" => cli_search(argument),
                _ => println!("{HELP}"),
            }
        }
        _ => println!("{HELP}"),
    }
}

// Create the required directory structure
fn cli_establish() {
    for dir in &[
        "feed",
        "feed/documents",
        "feed/export",
    ] {
        if fs::read_dir(dir).is_err() {
            println!("Creating {dir}...");
            fs::create_dir(dir).expect("Failed to create {dir}.");
        }
    }
}

// Create a new document
fn cli_create(filename: &str) {
    let folder_path = "feed/documents";
    let file_path = format!("{folder_path}/{filename}.md");

    if !fs::metadata(folder_path).is_ok() {
        eprintln!("The documents folder does not exist. Please run `adduce feed establish` to create the necessary file structure.");
        return;
    }

    if fs::metadata(&file_path).is_ok() {
        eprintln!("Document already exists: {file_path}.");
        return;
    }

    let initial_content = format!("# {filename}\n");
    if let Err(err) = fs::write(&file_path, initial_content) {
        eprintln!("Failed to create file {file_path}: {err}.");
        return;
    }

    println!("Created new file: {file_path}.");
}

// Remove a requested document
fn cli_remove(filename: &str) {
    let md_file_path = format!("feed/documents/{filename}.md");
    if let Err(error) = fs::remove_file(&md_file_path) {
        println!("Error removing source document {filename}: {error}.");
    } else {
        println!("Deleted source document '{filename}'.");
    }

    let html_file_path = format!("feed/export/{filename}.html");
    if let Err(error) = fs::remove_file(&html_file_path) {
        println!("Error removing exported document {filename}: {error}.");
    } else {
        println!("Deleted exported document '{filename}'.");
    }
}

// Edit a requested document
fn cli_edit(filename: &str) {
    let file_path = format!("feed/documents/{filename}.md");

    if fs::read(&file_path).is_err() {
        println!("No documents with that name.");
        return;
    }

        let editor_command = env::var("EDITOR").unwrap_or_else(|_| "notepad".to_string());

    Command::new(editor_command)
        .arg(file_path)
        .spawn()
        .expect("Failed to launch editor.")
        .wait()
        .expect("Editor exited with error.");
}

// Generate a HTML version of the input document
fn cli_export(document: &str) {
    let md_file_path = format!("feed/documents/{document}.md");
    if fs::metadata(&md_file_path).is_err() {
        println!("Input file '{document}' does not exist. Please create it first.");
        return;
    }

    let conf = match fs::read_to_string("feed/conf.toml") {
        Ok(content) => toml::from_str::<Conf>(&content).unwrap(),
        Err(e) => {
            println!("{e}\nYou must manually create a conf.toml file for your feed.");
            return;
        }
    };

    let text = Object {
        content_file: Some(format!("feed/documents/{document}.md")),
        format: Some(String::from("md")),
        ..Default::default()
    };

    let mut toml = conf;
    if toml.main.is_none() {
        toml.main = Some(Main { block: vec![text] });
    } else {
        toml.main.as_mut().unwrap().block.push(text);
    }

    if let Err(err) = fs::write(format!("feed/export/{document}.html"), toml.to_html()) {
        eprintln!("Failed to export {document}: {err}.");
        return;
    }

    println!("Successfully exported {document}.");
}

// Search documents
fn cli_search(keyword: &str) {
    let entries = fs::read_dir("feed/documents/")
        .expect("Failed to read documents directory.")
        .filter_map(|entry| entry.ok().map(|e| e.file_name().into_string().unwrap_or_default()));

        let mut found_results = false;

    for entry in entries {
        if entry.contains(keyword) {
            println!("{entry}");
            found_results = true;
        }
    }

    if !found_results {
        println!("No results found for '{keyword}'.");
    }
}

// TODO: Set item title to og:title in the header of the document
// TODO: Set item description to contents of <article> tag in the document

// Generate an RSS feed
fn cli_rss() {
    let mut items = Vec::new();

    for entry in fs::read_dir("feed/documents/").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        let content = fs::read_to_string(&path).unwrap_or_default();

        let item = ItemBuilder::default()
            .title(Some(path.file_name().unwrap().to_string_lossy().to_string()))
            .description(Some(content))
            .build();

        items.push(item);
    }

    let conf_content = match fs::read_to_string("feed/conf.toml") {
        Ok(content) => content,
        Err(e) => {
            println!("Error reading configuration file: {e}\nNo configuration file found.");
            return;
        }
    };

    let conf: Conf = match toml::from_str(&conf_content) {
        Ok(conf) => conf,
        Err(e) => {
            println!("Error parsing configuration file: {e}");
            return;
        }
    };

    if conf.title.is_none() || conf.link.is_none() || conf.description.is_none() {
        let mut missing_fields = Vec::new();

        if conf.title.is_none() {
            missing_fields.push("title");
        }
        if conf.link.is_none() {
            missing_fields.push("link");
        }
        if conf.description.is_none() {
            missing_fields.push("description");
        }

        println!("RSS feed not generated. Missing required fields: {}.", missing_fields.join(", "));
        return;
    }

    let channel = ChannelBuilder::default()
        .title(conf.title.unwrap())
        .link(conf.link.unwrap())
        .description(conf.description.unwrap())
        .language(conf.language)
        .copyright(conf.copyright)
        .managing_editor(conf.managing_editor)
        .webmaster(conf.webmaster)
        // TODO: Categories
        .ttl(conf.ttl)
        // TODO: Image
        // TODO: Skip Hours
        // TODO: Skip Days
        .generator(Some("Adduce".to_string()))
        .items(items)
        .build();

    if let Err(e) = fs::write("feed/export/feed.xml", channel.to_string()) {
        eprintln!("Failed to write RSS feed: {e}");
    } else {
        println!("RSS feed generated successfully.");
    }
}
