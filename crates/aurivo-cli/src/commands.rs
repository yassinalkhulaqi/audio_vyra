//! Command-line dispatcher.

use crate::{preset, state, wav};

const HELP: &str = "aurivo — AURIVO diagnostics\n\n\
Commands:\n\
  preset inspect <file.json>     Show preset name, params and chain\n\
  preset validate <file.json>    Check preset structure and values\n\
  preset list <directory>        List preset files and validity\n\
  state check <file.json>        List expected AppState fields\n\
  state diff <a.json> <b.json>   Show changed fields between snapshots\n\
  wav info <file.wav>            Print WAV header information\n\
  effects                        List the default effect chain\n\
  help                           Show this message\n";

pub fn run(args: &[String]) -> Result<(), String> {
    match args.first().map(String::as_str) {
        None | Some("help") | Some("--help") | Some("-h") => {
            print!("{HELP}");
            Ok(())
        }
        Some("preset") => {
            let sub = args.get(1).map(String::as_str).unwrap_or("");
            let path = args.get(2).ok_or_else(|| "usage: aurivo preset <inspect|validate|list> <path>".to_string())?;
            match sub {
                "inspect" => println!("{}", preset::inspect(path)?),
                "validate" => println!("{}", preset::validate(path)?),
                "list" => println!("{}", preset::list(path)?),
                _ => return Err(format!("unknown preset subcommand '{sub}'")),
            }
            Ok(())
        }
        Some("state") => {
            let sub = args.get(1).map(String::as_str).unwrap_or("");
            match sub {
                "check" => {
                    let path = args.get(2).ok_or_else(|| "usage: aurivo state check <file.json>".to_string())?;
                    println!("{}", state::check(path)?);
                }
                "diff" => {
                    let a = args.get(2).ok_or_else(|| "usage: aurivo state diff <a.json> <b.json>".to_string())?;
                    let b = args.get(3).ok_or_else(|| "usage: aurivo state diff <a.json> <b.json>".to_string())?;
                    println!("{}", state::diff(a, b)?);
                }
                _ => return Err(format!("unknown state subcommand '{sub}'")),
            }
            Ok(())
        }
        Some("wav") => {
            let sub = args.get(1).map(String::as_str).unwrap_or("");
            let path = args.get(2).ok_or_else(|| "usage: aurivo wav info <file.wav>".to_string())?;
            match sub {
                "info" => println!("{}", wav::describe(path)?),
                _ => return Err(format!("unknown wav subcommand '{sub}'")),
            }
            Ok(())
        }
        Some("effects") => {
            let engine = aurivo_audio_engine::AudioEngine::new();
            for (i, name) in engine.effect_names().iter().enumerate() {
                println!("  {:>2}. {}", i + 1, name);
            }
            Ok(())
        }
        Some(other) => Err(format!("unknown command '{other}'; try 'aurivo help'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn help_is_accepted() {
        assert!(run(&args(&["help"])).is_ok());
        assert!(run(&[]).is_ok());
    }

    #[test]
    fn unknown_command_is_rejected() {
        assert!(run(&args(&["nope"])).is_err());
    }

    #[test]
    fn missing_path_is_rejected() {
        assert!(run(&args(&["preset", "inspect"])).is_err());
        assert!(run(&args(&["state", "diff", "a.json"])).is_err());
    }

    #[test]
    fn effects_lists_default_chain() {
        assert!(run(&args(&["effects"])).is_ok());
    }
}
