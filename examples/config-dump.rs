use std::env;
use std::process;

use colored::Colorize;
use zocalo::{Configuration, PluginConfig, PluginDefinition};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("{}: {} <config-file>", "Usage".yellow(), args[0]);
        process::exit(1);
    }

    let config_path = &args[1];

    let config = match Configuration::from_file(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{}: {}", "Error".red().bold(), e);
            process::exit(1);
        }
    };

    println!("{} v{}", "Configuration".cyan().bold(), config.version());
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
                    println!(
                        "    {}: {}",
                        group.blue(),
                        plugins.join(", ").white()
                    );
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
                let (type_name, details) = match plugin {
                    PluginConfig::Graylog(g) => (
                        "graylog",
                        format!(
                            "{}://{}:{}",
                            format!("{:?}", g.protocol).to_lowercase(),
                            g.host,
                            g.port
                        ),
                    ),
                    PluginConfig::Logging(l) => (
                        "logging",
                        format!("{} loggers, {} verbose levels", l.loggers.len(), l.verbose.len()),
                    ),
                    PluginConfig::Storage(s) => ("storage", format!("{} keys", s.values.len())),
                    PluginConfig::Transport(t) => {
                        ("transport", format!("default: {}", t.default))
                    }
                    PluginConfig::Slurm(s) => ("slurm", s.url.clone()),
                    PluginConfig::RabbitMQApi(r) => ("rabbitmqapi", r.base_url.clone()),
                    PluginConfig::Smtp(s) => ("smtp", format!("{}:{}", s.host, s.port)),
                    PluginConfig::Jmx(j) => ("jmx", format!("{}:{}", j.host, j.port)),
                };
                println!(
                    "  {} {} {} {}",
                    name.yellow(),
                    "->".dimmed(),
                    type_name.cyan(),
                    format!("({})", details).dimmed()
                );
            }
        }
    }
}
