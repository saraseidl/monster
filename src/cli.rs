use anyhow::{anyhow, Context, Result};
use clap::{crate_authors, crate_description, crate_version, App, AppSettings, Arg, ArgMatches};
use const_format::formatcp;
use lazy_static::lazy_static;
use monster::{
    engine::{
        rarity_simulation::{defaults as rarity_defaults, MeanType},
        symbolic_execution::defaults as symbolic_defaults,
    },
    path_exploration::ExplorationStrategyType,
    solver::SolverType,
};
use std::str::FromStr;
use strum::{EnumString, EnumVariantNames, IntoStaticStr, VariantNames};

#[derive(Debug, PartialEq, EnumString, EnumVariantNames, IntoStaticStr)]
#[strum(serialize_all = "kebab_case")]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

lazy_static! {
    static ref COPY_INIT_RATIO: String = format!("{}", rarity_defaults::COPY_INIT_RATIO);
}

pub fn args() -> App<'static, 'static> {
    App::new("Monster")
        .version(crate_version!())
        .author(crate_authors!(", "))
        .about(crate_description!())
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .help("configure logging level to use")
                .takes_value(true)
                .value_name("LEVEL")
                .possible_values(&LogLevel::VARIANTS)
                .default_value(LogLevel::Info.into())
                .global(true),
        )
        .subcommand(
            App::new("disassemble")
                .about("Disassemble a RISC-V ELF binary")
                .arg(
                    Arg::with_name("input-file")
                        .value_name("FILE")
                        .help("Binary file to be disassembled")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .subcommand(
            App::new("cfg")
                .about("Generate control flow graph from RISC-U ELF binary")
                .arg(
                    Arg::with_name("input-file")
                        .help("Source RISC-U binary to be analyzed")
                        .takes_value(true)
                        .value_name("FILE")
                        .required(true),
                )
                .arg(
                    Arg::with_name("output-file")
                        .help("Output file to write to")
                        .short("o")
                        .long("output-file")
                        .takes_value(true)
                        .value_name("FILE")
                        .default_value("cfg.dot"),
                )
                .arg(
                    Arg::with_name("distances")
                        .help("Compute also shortest path distances from exit")
                        .short("d")
                        .long("distances"),
                ),
        )
        .subcommand(
            App::new("execute")
                .about("Symbolically execute a RISC-U ELF binary")
                .arg(
                    Arg::with_name("input-file")
                        .help("RISC-U ELF binary to be executed")
                        .takes_value(true)
                        .value_name("FILE")
                        .required(true),
                )
                .arg(
                    Arg::with_name("solver")
                        .help("SMT solver")
                        .short("s")
                        .long("solver")
                        .takes_value(true)
                        .value_name("SOLVER")
                        .possible_values(&SolverType::VARIANTS)
                        .default_value(SolverType::Monster.into()),
                )
                .arg(
                    Arg::with_name("max-execution-depth")
                        .help("Number of instructions, where the path execution will be aborted")
                        .short("d")
                        .long("execution-depth")
                        .takes_value(true)
                        .value_name("NUMBER")
                        .default_value(formatcp!("{}", symbolic_defaults::MAX_EXECUTION_DEPTH))
                        .validator(is::<u64>),
                )
                .arg(
                    Arg::with_name("memory")
                        .help("Amount of memory to be used per execution context in megabytes [possible_values: 1 .. 1024]")
                        .short("m")
                        .long("memory")
                        .takes_value(true)
                        .value_name("NUMBER")
                        .default_value(formatcp!("{}", symbolic_defaults::MEMORY_SIZE.0 / bytesize::MIB))
                        .validator(is_valid_memory_size),
                )
                .arg(
                    Arg::with_name("strategy")
                    .help("Path exploration strategy to use when exploring state search space")
                    .long("strategy")
                    .takes_value(true)
                    .value_name("STRATEGY")
                    .default_value(ExplorationStrategyType::ShortestPaths.into())
                    .possible_values(ExplorationStrategyType::VARIANTS)
                ),
        )
        .subcommand(
            App::new("rarity")
                .about("Performs rarity simulation on a RISC-U ELF binary")
                .arg(
                    Arg::with_name("input-file")
                        .help("Source RISC-U binary to be analyzed")
                        .takes_value(true)
                        .value_name("FILE")
                        .required(true),
                )
                .arg(
                    Arg::with_name("memory")
                        .help("Amount of memory to be used per execution context in megabytes [possible_values: 1 .. 1024]")
                        .short("m")
                        .long("memory")
                        .takes_value(true)
                        .value_name("NUMBER")
                        .default_value(formatcp!("{}", rarity_defaults::MEMORY_SIZE.0 / bytesize::MIB))
                        .validator(is_valid_memory_size),
                )
                .arg(
                    Arg::with_name("step-size")
                        .help("Instructions to be executed for each round")
                        .long("step-size")
                        .takes_value(true)
                        .value_name("NUMBER")
                        .default_value(formatcp!("{}", rarity_defaults::STEP_SIZE))
                        .validator(is::<u64>),
                )
                .arg(
                    Arg::with_name("states")
                        .help("Number of distinct states")
                        .long("states")
                        .takes_value(true)
                        .value_name("NUMBER")
                        .default_value(formatcp!("{}", rarity_defaults::AMOUNT_OF_STATES))
                        .validator(is::<usize>),
                )
                .arg(
                    Arg::with_name("selection")
                    .help("Number of runs to select in every iteration")
                    .short("s")
                    .long("selection")
                    .takes_value(true)
                    .value_name("NUMBER")
                    .default_value(formatcp!("{}", rarity_defaults::SELECTION))
                    .validator(is::<usize>))
                .arg(
                    Arg::with_name("iterations")
                    .help("Iterations of rarity simulation to run")
                    .short("i")
                    .long("iterations")
                    .takes_value(true)
                    .value_name("NUMBER")
                    .default_value(formatcp!("{}", rarity_defaults::ITERATIONS))
                    .validator(is::<u64>))
                .arg(
                    Arg::with_name("copy-init-ratio")
                        .help("Determines how much new states are copied instead of started from the beginning")
                        .long("copy-init-ratio")
                        .takes_value(true)
                        .value_name("RATIO")
                        .default_value(COPY_INIT_RATIO.as_str())
                        .validator(is_ratio)
                    )
                .arg(
                    Arg::with_name("mean")
                    .help("The average to be used for the counts")
                    .long("mean")
                    .takes_value(true)
                    .value_name("MEAN")
                    .possible_values(&MeanType::VARIANTS)
                    .default_value(rarity_defaults::MEAN_TYPE.into())
                    )
        )
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .global_setting(AppSettings::GlobalVersion)
}

pub fn expect_arg<T: FromStr>(m: &ArgMatches, arg: &str) -> Result<T>
where
    <T as FromStr>::Err: Send + Sync + std::error::Error + 'static,
{
    m.value_of(arg)
        .ok_or_else(|| anyhow!("argument \"{}\" has to be set in CLI at all times", arg))
        .and_then(|s| {
            T::from_str(s).with_context(|| format!("argument \"{}\" has wrong format", arg))
        })
}

fn is<T: FromStr>(v: String) -> Result<(), String>
where
    <T as FromStr>::Err: std::fmt::Display,
{
    v.parse::<T>().map(|_| ()).map_err(|e| e.to_string())
}

fn is_valid_memory_size(v: String) -> Result<(), String> {
    is::<u64>(v.clone()).and_then(|_| {
        let memory_size = v.parse::<u64>().expect("have checked that already");

        let valid_range = 1_u64..=1024_u64;

        if valid_range.contains(&memory_size) {
            Ok(())
        } else {
            Err(String::from("memory size has to be in range: 1 - 1024"))
        }
    })
}

fn is_ratio(v: String) -> Result<(), String> {
    let valid_range = 0.0_f64..=1.0f64;

    match v.parse::<f64>() {
        Ok(ratio) => {
            if valid_range.contains(&ratio) {
                Ok(())
            } else {
                Err("Expected range between 0.0 and 1.0".to_string())
            }
        }
        Err(err) => Err(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_matches<F>(a: Vec<&str>, f: F)
    where
        F: Fn(&ArgMatches),
    {
        let matches = args().get_matches_from(a.clone());

        f(matches.subcommand_matches(a[1]).unwrap())
    }

    #[test]
    fn test_execute_defaults_are_set() {
        with_matches(vec!["monster", "execute", "file.o"], |m| {
            assert!(m.is_present("memory"), "Default memory size is set");
            assert!(
                m.is_present("max-execution-depth"),
                "Default execution depth is set"
            );
            assert!(m.is_present("solver"), "Default solver is set");
        });
    }

    #[test]
    fn test_execute_memory_size_argument() {
        assert!(
            args()
                .get_matches_from_safe(vec!["monster", "execute", "-m", "0", "file.o"])
                .is_err(),
            "Memory size 0 is invalid"
        );

        assert!(
            args()
                .get_matches_from_safe(vec!["monster", "execute", "-m", "-23424", "file.o"])
                .is_err(),
            "Negative memory size is invalid"
        );

        assert!(
            args()
                .get_matches_from_safe(vec!["monster", "execute", "-m", "23424", "file.o"])
                .is_err(),
            "memory size is invalid (out of range)"
        );
    }
}
