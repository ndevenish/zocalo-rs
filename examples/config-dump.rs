use std::process;

use clap::Parser;
use colored::Colorize;
use zocalo::{
    Configuration, GraylogConfig, JmxConfig, LoggingConfig, PluginDefinition, RabbitMQApiConfig,
    RabbitMQConfig, SlurmConfig, SmtpConfig, TransportConfig,
};

/// Dump a Zocalo configuration file
#[derive(Parser)]
#[command(name = "config-dump")]
struct Args {
    /// Path to the configuration file
    config_file: String,
}

fn main() {
    let args = Args::parse();

    let config = match Configuration::from_file(&args.config_file) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            process::exit(1);
        }
    };
    println!();

    // Print environments
    println!("{}:", "Environments".green().bold());
    let mut envs: Vec<_> = config.environments().collect();
    envs.sort();
    for env_name in envs {
        let env = config.get_environment(env_name).unwrap();
        if let Some(alias) = env.alias() {
            println!(
                "  {} {} {} {}",
                env_name.yellow(),
                "->".dimmed(),
                alias.cyan(),
                "(alias)".dimmed()
            );
        } else {
            let groups: Vec<_> = env.groups().collect();
            let plugin_count: usize = groups
                .iter()
                .filter_map(|g| env.get_group(g))
                .map(|p| p.len())
                .sum();
            println!(
                "  {} {}",
                env_name.yellow(),
                format!("({} groups, {} plugins)", groups.len(), plugin_count).dimmed()
            );
            for group in groups {
                if let Some(plugins) = env.get_group(group) {
                    println!("    {}: {}", group.blue(), plugins.join(", ").white());
                }
            }
        }
    }
    println!();

    // Print plugin definitions
    println!("{}:", "Plugin Definitions".green().bold());
    let mut plugins: Vec<_> = config.plugin_definitions().collect();
    plugins.sort_by_key(|(name, _)| *name);
    for (name, def) in plugins {
        match def {
            PluginDefinition::Unresolved(path) => {
                println!(
                    "  {} {} {} {}",
                    name.yellow(),
                    "->".dimmed(),
                    path.display().to_string().white(),
                    "(external)".magenta()
                );
            }
            PluginDefinition::Resolved(plugin) => {
                let details = match plugin.plugin.as_str() {
                    "pika" => {
                        let r: RabbitMQConfig =
                            serde_yaml::from_value(plugin.values.clone()).unwrap();
                        let port = r.port.map(|p| p.to_string()).unwrap_or_default();
                        format!("{}:{}", r.host.join(","), port)
                    }
                    "transport" => format!(
                        "default: {}",
                        serde_yaml::from_value::<TransportConfig>(plugin.values.clone())
                            .unwrap()
                            .default
                    ),
                    "graylog" => {
                        let g: GraylogConfig =
                            serde_yaml::from_value(plugin.values.clone()).unwrap();
                        format!(
                            "{}://{}:{}",
                            format!("{:?}", g.protocol).to_lowercase(),
                            g.host,
                            g.port
                        )
                    }
                    "slurm" => serde_yaml::from_value::<SlurmConfig>(plugin.values.clone())
                        .unwrap()
                        .url
                        .clone(),
                    "rabbitmqapi" => {
                        serde_yaml::from_value::<RabbitMQApiConfig>(plugin.values.clone())
                            .unwrap()
                            .base_url
                            .join(",")
                    }
                    "smtp" => {
                        let s: SmtpConfig = serde_yaml::from_value(plugin.values.clone()).unwrap();
                        format!("{}:{}", s.host, s.port)
                    }
                    "jmx" => {
                        let j: JmxConfig = serde_yaml::from_value(plugin.values.clone()).unwrap();
                        format!("{}:{}", j.host, j.port)
                    }
                    "logging" => {
                        let l: LoggingConfig =
                            serde_yaml::from_value(plugin.values.clone()).unwrap();
                        format!(
                            "{} loggers, {} verbose levels",
                            l.loggers.len(),
                            l.verbose.len()
                        )
                    }
                    _ => format!("{} keys", plugin.values.as_mapping().unwrap().len()),
                };
                println!(
                    "  {} {} {} {}",
                    name.yellow(),
                    "->".dimmed(),
                    plugin.plugin.cyan(),
                    format!("({})", details).dimmed()
                );
            }
        }
    }
}
