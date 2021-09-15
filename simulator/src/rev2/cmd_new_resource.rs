use clap::{crate_version, App, Arg, ArgMatches, SubCommand};
use radix_engine::execution::*;
use scrypto::buffer::*;
use scrypto::types::*;
use scrypto::utils::*;
use uuid::Uuid;

use crate::ledger::*;
use crate::rev2::*;

const ARG_TRACE: &str = "TRACE";
const ARG_SYMBOL: &str = "SYMBOL";
const ARG_NAME: &str = "NAME";
const ARG_DESCRIPTION: &str = "DESCRIPTION";
const ARG_URL: &str = "URL";
const ARG_ICON_URL: &str = "ICON_URL";
const ARG_SUPPLY: &str = "SUPPLY";
const ARG_MINTER: &str = "MINTER";

/// Constructs a `new-resource` subcommand.
pub fn make_new_resource_cmd<'a, 'b>() -> App<'a, 'b> {
    SubCommand::with_name(CMD_CREATE_RESOURCE)
        .about("Creates a new resource")
        .version(crate_version!())
        .arg(
            Arg::with_name(ARG_TRACE)
                .short("t")
                .long("trace")
                .help("Turns on tracing."),
        )
        .arg(
            Arg::with_name(ARG_SYMBOL)
                .long("symbol")
                .takes_value(true)
                .help("Specify the symbol.")
                .required(true),
        )
        .arg(
            Arg::with_name(ARG_NAME)
                .long("name")
                .takes_value(true)
                .help("Specify the name.")
                .required(false),
        )
        .arg(
            Arg::with_name(ARG_DESCRIPTION)
                .long("description")
                .takes_value(true)
                .help("Specify the description.")
                .required(false),
        )
        .arg(
            Arg::with_name(ARG_URL)
                .long("url")
                .takes_value(true)
                .help("Specify the URL.")
                .required(false),
        )
        .arg(
            Arg::with_name(ARG_ICON_URL)
                .long("icon-url")
                .takes_value(true)
                .help("Specify the icon URL.")
                .required(false),
        )
        .arg(
            Arg::with_name(ARG_SUPPLY)
                .long("supply")
                .takes_value(true)
                .help("Specify the total supply.")
                .required(false),
        )
        .arg(
            Arg::with_name(ARG_MINTER)
                .long("minter")
                .takes_value(true)
                .help("Specify the minter.")
                .required(false),
        )
}

/// Handles a `new-resource` request.
pub fn handle_new_resource(matches: &ArgMatches) -> Result<(), Error> {
    let trace = matches.is_present(ARG_TRACE);
    let symbol = matches.value_of(ARG_SYMBOL).unwrap_or("");
    let name = matches.value_of(ARG_NAME).unwrap_or("");
    let description = matches.value_of(ARG_DESCRIPTION).unwrap_or("");
    let url = matches.value_of(ARG_URL).unwrap_or("");
    let icon_url = matches.value_of(ARG_ICON_URL).unwrap_or("");
    let supply = matches
        .value_of(ARG_SUPPLY)
        .and_then(|v| U256::from_dec_str(v).ok());
    let minter = matches
        .value_of(ARG_MINTER)
        .and_then(|v| v.parse::<Address>().ok());
    if !(supply.is_some() ^ minter.is_some()) {
        return Err(Error::MissingArgument(
            "Either supply or minter must be set".to_owned(),
        ));
    }

    match get_config(CONF_DEFAULT_ACCOUNT)? {
        Some(a) => {
            let account: Address = a.as_str().parse().map_err(Error::InvalidAddress)?;
            let tx_hash = sha256(Uuid::new_v4().to_string());
            let mut ledger = FileBasedLedger::new(get_data_dir()?);
            let mut runtime = Runtime::new(tx_hash, &mut ledger);

            let mut process = Process::new(0, trace, &mut runtime);
            let output = process
                .prepare_call_method(
                    account,
                    if supply.is_some() {
                        "new_resource_fixed".to_owned()
                    } else {
                        "new_resource_mutable".to_owned()
                    },
                    vec![
                        scrypto_encode(symbol),
                        scrypto_encode(name),
                        scrypto_encode(description),
                        scrypto_encode(url),
                        scrypto_encode(icon_url),
                        if supply.is_some() {
                            scrypto_encode(&supply.unwrap())
                        } else {
                            scrypto_encode(&minter.unwrap())
                        },
                    ],
                )
                .and_then(|target| process.run(target))
                .map_err(Error::TxnExecutionError)?;
            process.finalize().map_err(Error::TxnExecutionError)?;
            let resource: Address = scrypto_decode(&output).map_err(Error::DataError)?;

            runtime.flush();
            println!("New resource: {}", resource);

            Ok(())
        }
        None => Err(Error::NoDefaultAccount),
    }
}