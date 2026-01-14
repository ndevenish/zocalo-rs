use std::env;
use std::process;

use zocalo::{Configuration, PluginConfig, PluginDefinition};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        eprintln!("Usage: {} <config-file>", args[0]);
        process::exit(1);
    }

    let config_path = &args[1];

    let config = match Configuration::from_file(config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error loading configuration: {}", e);
            process::exit(1);
        }
    };

    println!("Configuration v{}", config.version());
    println!();

    // Print environments
    println!("Environments:");
    let mut envs: Vec<_> = config.environments().collect();
    envs.sort();
    for env_name in envs {
        let env = config.get_environment(env_name).unwrap();
        if let Some(alias) = env.alias() {
            println!("  {} -> {} (alias)", env_name, alias);
        } else {
            let groups: Vec<_> = env.groups().collect();
            let plugin_count: usize = groups.iter()
                .filter_map(|g| env.get_group(g))
                .map(|p| p.len())
                .sum();
            println!("  {} ({} groups, {} plugins)", env_name, groups.len(), plugin_count);
            for group in groups {
                if let Some(plugins) = env.get_group(group) {
                    println!("    {}: {}", group, plugins.join(", "));
                }
            }
        }
    }
    println!();

    // Print plugin definitions
    println!("Plugin Definitions:");
    let mut plugins: Vec<_> = config.plugin_definitions().collect();
    plugins.sort_by_key(|(name, _)| *name);
    for (name, def) in plugins {
        match def {
            PluginDefinition::Unresolved(path) => {
                println!("  {} -> {} (external)", name, path.display());
            }
            PluginDefinition::Resolved(plugin) => {
                let type_name = match plugin {
                    PluginConfig::Graylog(g) => {
                        format!("graylog ({}://{}:{})",
                            format!("{:?}", g.protocol).to_lowercase(),
                            g.host, g.port)
                    }
                    PluginConfig::Logging(l) => {
                        format!("logging ({} loggers, {} verbose levels)",
                            l.loggers.len(), l.verbose.len())
                    }
                    PluginConfig::Storage(s) => {
                        format!("storage ({} keys)", s.values.len())
                    }
                    PluginConfig::Transport(t) => {
                        format!("transport (default: {})", t.default)
                    }
                    PluginConfig::Slurm(s) => {
                        format!("slurm ({})", s.url)
                    }
                    PluginConfig::RabbitMQApi(r) => {
                        format!("rabbitmqapi ({})", r.base_url)
                    }
                    PluginConfig::Smtp(s) => {
                        format!("smtp ({}:{})", s.host, s.port)
                    }
                    PluginConfig::Jmx(j) => {
                        format!("jmx ({}:{})", j.host, j.port)
                    }
                };
                println!("  {} -> {}", name, type_name);
            }
        }
    }
}
