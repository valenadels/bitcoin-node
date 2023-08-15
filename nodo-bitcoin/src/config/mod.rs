use std::io::{BufRead, BufReader};
use std::{env, fs::File};

use crate::constants::{
    STARTING_DATE, {DEFAULT_CONFIG, PATH_LOG},
};
use crate::node_error::NodeError;
use crate::ui::ui_message::UIMessage;

/// Returns the path to the configuration file.
///
/// This function takes a default file path and checks if the user has provided an alternate
/// file path as a command line argument. If an argument is provided, the function returns that
/// path. Otherwise, it returns the default file path.
fn get_config_path(default_file: String) -> String {
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        return String::from(&args[1]);
    }
    default_file
}
/// Load application configuration from a file and set environment variables accordingly.
///
/// The function reads a configuration file, specified either by the `CONFIG` environment variable
/// or by a default path. The configuration file should contain key-value pairs, with each line
/// representing a separate pair. The function sets environment variables for each key-value pair
/// found in the configuration file.
///
/// # Errors
///
/// The function returns an error if the configuration file could not be opened or read.
///
pub fn load_app_config(ui_sender: Option<&glib::Sender<UIMessage>>) -> Result<(), NodeError> {
    let path_config = get_config_path(DEFAULT_CONFIG.to_string());
    let file = File::open(path_config)
        .map_err(|_| NodeError::FailedToOpenFile("Failed to open config file".to_string()))?;
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line_content =
            line.map_err(|_| NodeError::FailedToRead("Failed to read line".to_string()))?;
        let (key, value) = parse_line(&line_content)?;
        if key == STARTING_DATE {
            if let Some(sender) = ui_sender {
                sender
                    .send(UIMessage::StartingDate(value.to_string()))
                    .expect("Failed to send starting date");
            }
        }
        configure_environ_var(key, value);
    }
    let path_log = std::env::var(PATH_LOG).map_err(|_| {
        NodeError::EnvironVarNotFound("PATH_LOG not found in env vars to delete".to_string())
    })?;
    let _ = std::fs::remove_file(path_log);

    Ok(())
}

/// Returns the directory path for the key passed as an argument from the environment variables.
pub fn obtain_dir_path(config_key: String) -> Result<String, NodeError> {
    let directory = match std::env::var(&config_key) {
        Ok(dir) => dir,
        Err(_) => {
            println!("Error obtaining the directory path {}", config_key);
            return Err(NodeError::EnvironVarNotFound(config_key));
        }
    };
    Ok(directory)
}

/// Parse a key-value pair from a line of text.
///
/// The function takes a line of text containing a single key-value pair in the format `key=value`.
/// The function returns a tuple containing the key and value as separate strings.
///
/// # Errors
///
/// The function returns an error if the input line is not formatted correctly, i.e. if it does not
/// contain an equal sign or if either the key or value is missing.
pub fn parse_line(line: &str) -> Result<(&str, &str), NodeError> {
    let mut split = line.split('=');
    let key = split
        .next()
        .ok_or("Invalid config file format")
        .map_err(|_| NodeError::FailedToParse("Failed to parse key from config".to_string()))?;
    let value = split
        .next()
        .ok_or("Invalid config file format")
        .map_err(|_| NodeError::FailedToParse("Failed to parse value from config".to_string()))?;
    Ok((key, value))
}

/// Set an environment variable with the given key and value.
fn configure_environ_var(key: &str, value: &str) {
    env::set_var(key, value);
}

#[cfg(test)]
mod test {
    use std::env;

    use crate::{
        config::{load_app_config, parse_line},
        node_error::NodeError,
    };

    #[test]
    fn test_load_app_config() {
        load_app_config(None).unwrap();
        assert!(env::var("DNS").is_ok());
    }

    #[test]
    fn test_parse_line() -> Result<(), NodeError> {
        let line = "DNS=seed.testnet.bitcoin.sprovoost.nl";
        let (key, value) = parse_line(line)?;
        assert_eq!(key, "DNS");
        assert_eq!(value, "seed.testnet.bitcoin.sprovoost.nl");
        Ok(())
    }

    #[test]
    fn test_parse_line_error() {
        let line = "DNS";
        parse_line(line).expect_err("Invalid config file format");
    }
}
