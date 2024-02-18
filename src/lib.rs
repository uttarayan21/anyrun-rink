use abi_stable::std_types::{ROption, RString, RVec};
use anyrun_plugin::*;
use error_stack::{Report, ResultExt};
use rink_core::{ast, parsing, CURRENCY_FILE};
use std::path::PathBuf;

#[derive(serde::Deserialize, Default)]
pub struct RinkConfig {
    currency: Option<PathBuf>,
}

#[derive(Debug)]
pub struct RinkError;
impl core::fmt::Display for RinkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "RinkError")
    }
}

impl std::error::Error for RinkError {}

#[init]
fn init(config_dir: RString) -> rink_core::Context {
    try_init(config_dir)
        .map_err(|e| {
            eprintln!("{}", e);
            e
        })
        .expect("Rink failed to initialize")
}

fn try_init(config_dir: RString) -> Result<rink_core::Context, Report<RinkError>> {
    use rink_core::loader::gnu_units;
    let mut ctx = rink_core::Context::new();

    let units = gnu_units::parse_str(
        rink_core::DEFAULT_FILE
            .ok_or_else(|| RinkError)
            .attach_printable("Default units file is missing.")?,
    );
    let dates = parsing::datetime::parse_datefile(rink_core::DATES_FILE);

    let mut currency_defs = Vec::new();

    match reqwest::blocking::get("https://rinkcalc.app/data/currency.json") {
        Ok(response) => match response.json::<ast::Defs>() {
            Ok(mut live_defs) => {
                currency_defs.append(&mut live_defs.defs);
            }
            Err(why) => println!("Error parsing currency json: {}", why),
        },
        Err(why) => println!("Error fetching up-to-date currency conversions: {}", why),
    }

    let config_path = PathBuf::from(config_dir.into_string()).join("rink.ron");
    let config = std::fs::read(&config_path)
        .change_context_lazy(|| RinkError)
        .attach_printable("Rink config file couln't be read. Using default config.")
        .map_err(|e| {
            eprintln!("{}", e);
            e
        })
        .and_then(|data| {
            ron::de::from_bytes::<RinkConfig>(&data)
                .change_context_lazy(|| RinkError)
                .attach_printable("Rink config file malformed. Using default config.")
                .map_err(|e| {
                    eprintln!("{}", e);
                    e
                })
        })
        .unwrap_or_default();
    if let Some(currency_path) = config.currency {
        let units = std::fs::read_to_string(currency_path)
            .change_context_lazy(|| RinkError)
            .attach_printable("Currency file couldn't be read.")
            .map_err(|e| {
                eprintln!("{}", e);
                e
            })
            .map(|data| gnu_units::parse_str(&data));
        if let Ok(mut units) = units {
            currency_defs.append(&mut units.defs);
        } else {
            eprintln!("Extra Currency file couldn't be parsed.");
        }
    }

    currency_defs.append(&mut gnu_units::parse_str(CURRENCY_FILE).defs);

    ctx.load(units);
    ctx.load(ast::Defs {
        defs: currency_defs,
    });
    ctx.load_dates(dates);

    Ok(ctx)
}

#[info]
fn info() -> PluginInfo {
    PluginInfo {
        name: "Rink".into(),
        icon: "accessories-calculator".into(),
    }
}

#[get_matches]
fn get_matches(input: RString, ctx: &mut rink_core::Context) -> RVec<Match> {
    match rink_core::one_line(ctx, &input) {
        Ok(result) => {
            let (title, desc) = parse_result(result);
            vec![Match {
                title: title.into(),
                description: desc.map(RString::from).into(),
                use_pango: false,
                icon: ROption::RNone,
                id: ROption::RNone,
            }]
            .into()
        }
        Err(_) => RVec::new(),
    }
}

#[handler]
fn handler(selection: Match) -> HandleResult {
    HandleResult::Copy(selection.title.into_bytes())
}

/// Extracts the title and description from `rink` result.
/// The description is anything inside brackets from `rink`, if present.
fn parse_result(result: String) -> (String, Option<String>) {
    result
        .split_once(" (")
        .map(|(title, desc)| {
            (
                title.to_string(),
                Some(desc.trim_end_matches(')').to_string()),
            )
        })
        .unwrap_or((result, None))
}
