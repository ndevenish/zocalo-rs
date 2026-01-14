use std::process;

use clap::{ArgAction, Parser};
use colored::Colorize;
use zocalo::{ActivatedEnvironment, Configuration};

/// Activate and display a Zocalo environment
#[derive(Parser)]
#[command(name = "activate")]
struct Args {
    /// Path to the configuration file (or set ZOCALO_CONFIG)
    #[arg(short, long)]
    config: Option<String>,

    /// Environment to activate (or set ZOCALO_DEFAULT_ENV)
    #[arg(short, long, action = ArgAction::Append)]
    environment: Vec<String>,
}

fn main() {
    let args = Args::parse();

    let config = match &args.config {
        Some(path) => Configuration::from_file(path),
        None => Configuration::from_env(),
    };

    let mut config = match config {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            process::exit(1);
        }
    };

    match config.activate(Some(&args.environment)) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            process::exit(1);
        }
    };

    let config = match config.resolve() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            process::exit(1);
        }
    };

    print_activated(&config);
}

fn print_activated(activated: &ActivatedEnvironment) {
    println!(
        "{}: {}",
        "Environments".cyan().bold(),
        activated
            .environments
            .iter()
            .map(|f| f.yellow().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!();

    // Graylog
    if let Some(graylog) = &activated.graylog {
        println!("{}:", "Graylog".green().bold());
        println!(
            "  {} {}://{}:{}",
            "endpoint:".dimmed(),
            format!("{:?}", graylog.protocol).to_lowercase().cyan(),
            graylog.host.white(),
            graylog.port.to_string().white()
        );
        println!();
    }

    // Logging
    if let Some(logging) = &activated.logging {
        println!("{}:", "Logging".green().bold());
        if let Some(root) = &logging.root {
            println!(
                "  {} {}",
                "root level:".dimmed(),
                root.level.as_deref().unwrap_or("(unset)").cyan()
            );
        }
        if !logging.loggers.is_empty() {
            println!("  {}:", "loggers".dimmed());
            let mut loggers: Vec<_> = logging.loggers.iter().collect();
            loggers.sort_by_key(|(name, _)| *name);
            for (name, config) in loggers {
                println!(
                    "    {} {}",
                    format!("{}:", name).yellow(),
                    config.level.as_deref().unwrap_or("(unset)").white()
                );
            }
        }
        println!(
            "  {} {}",
            "verbose levels:".dimmed(),
            logging.verbose.len().to_string().white()
        );
        println!();
    }

    // Transport
    if let Some(transport) = &activated.transport {
        println!("{}:", "Transport".green().bold());
        println!("  {} {}", "default:".dimmed(), transport.default.cyan());
        println!();
    }

    // Slurm
    if let Some(slurm) = &activated.slurm {
        println!("{}:", "Slurm".green().bold());
        println!("  {} {}", "url:".dimmed(), slurm.url.cyan());
        println!(
            "  {} {}",
            "api_version:".dimmed(),
            slurm.api_version.white()
        );
        if let Some(user) = &slurm.user {
            println!("  {} {}", "user:".dimmed(), user.white());
        }
        println!();
    }

    // RabbitMQ (AMQP)
    if let Some(rabbitmq) = &activated.rabbitmq {
        println!("{}:", "RabbitMQ".green().bold());
        let port_str = rabbitmq
            .port
            .map(|p| p.to_string())
            .unwrap_or_else(|| "(default)".to_string());
        println!(
            "  {} {}:{}",
            "server:".dimmed(),
            rabbitmq
                .host
                .iter()
                .map(|s| s.cyan().to_string())
                .collect::<Vec<_>>()
                .join(", "),
            port_str.white()
        );
        println!("  {} {}", "username:".dimmed(), rabbitmq.username.white());
        println!("  {} {}", "vhost:".dimmed(), rabbitmq.vhost.white());
        println!();
    }

    // RabbitMQ API
    if let Some(rabbitmq) = &activated.rabbitmqapi {
        println!("{}:", "RabbitMQ API".green().bold());
        println!(
            "  {} {}",
            "base_url:".dimmed(),
            rabbitmq
                .base_url
                .iter()
                .map(|s| s.cyan().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!("  {} {}", "username:".dimmed(), rabbitmq.username.white());
        println!("  {} {}", "vhost:".dimmed(), rabbitmq.vhost.white());
        println!();
    }

    // SMTP
    if let Some(smtp) = &activated.smtp {
        println!("{}:", "SMTP".green().bold());
        println!(
            "  {} {}:{}",
            "server:".dimmed(),
            smtp.host.cyan(),
            smtp.port.to_string().white()
        );
        if let Some(from) = &smtp.from_address {
            println!("  {} {}", "from:".dimmed(), from.white());
        }
        println!();
    }

    // JMX
    if let Some(jmx) = &activated.jmx {
        println!("{}:", "JMX".green().bold());
        println!(
            "  {} {}:{}",
            "server:".dimmed(),
            jmx.host.cyan(),
            jmx.port.to_string().white()
        );
        println!("  {} {}", "base_url:".dimmed(), jmx.base_url.white());
        println!("  {} {}", "username:".dimmed(), jmx.username.white());
        println!();
    }

    // Storage
    if !activated.storage.is_empty() {
        println!("{}:", "Storage".green().bold());
        let mut keys: Vec<_> = activated.storage.keys().collect();
        keys.sort();
        for key in keys {
            let value = &activated.storage[key];
            let value_str = format_yaml_value(value);
            if value_str.contains('\n') {
                println!("  {}:", key.yellow());
                for line in value_str.lines() {
                    println!("    {}", line.white());
                }
            } else {
                println!("  {} {}", format!("{}:", key).yellow(), value_str.white());
            }
        }
        println!();
    }
    if !activated.unknown.is_empty() {
        println!("{}:", "Unknown Plugins".green().bold());
        let mut unknown = activated.unknown.clone();
        unknown.sort_by_key(|x| x.plugin.clone());
        for obj in unknown.iter() {
            println!("  {}:", obj.plugin.yellow());
            for (key, yaml) in obj.values.iter() {
                let value_str = format_yaml_value(yaml);
                if value_str.contains('\n') {
                    println!("    {}:", key.white());
                    for line in value_str.lines() {
                        println!("      {}", line.white());
                    }
                } else {
                    println!("    {} {}", format!("{}:", key).white(), value_str.white());
                }
            }
        }
    }
}

fn format_yaml_value(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Null => "(null)".to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Sequence(seq) => {
            if seq.len() <= 3
                && seq
                    .iter()
                    .all(|v| matches!(v, serde_yaml::Value::String(_)))
            {
                // Short list of strings - inline
                let items: Vec<_> = seq
                    .iter()
                    .map(|v| match v {
                        serde_yaml::Value::String(s) => s.clone(),
                        _ => format_yaml_value(v),
                    })
                    .collect();
                format!("[{}]", items.join(", "))
            } else {
                // Multi-line
                let items: Vec<_> = seq
                    .iter()
                    .map(|v| format!("- {}", format_yaml_value(v)))
                    .collect();
                items.join("\n")
            }
        }
        serde_yaml::Value::Mapping(map) => {
            let items: Vec<_> = map
                .iter()
                .map(|(k, v)| {
                    let key = format_yaml_value(k);
                    let val = format_yaml_value(v);
                    if val.contains('\n') {
                        format!("{}:\n  {}", key, val.replace('\n', "\n  "))
                    } else {
                        format!("{}: {}", key, val)
                    }
                })
                .collect();
            items.join("\n")
        }
        serde_yaml::Value::Tagged(tagged) => format_yaml_value(&tagged.value),
    }
}
