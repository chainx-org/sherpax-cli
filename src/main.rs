use serde_json::json;
use sp_core::crypto::{AccountId32, Ss58Codec};
use subxt::{BlockNumber, ClientBuilder};
use structopt::StructOpt;

const MAX_PAG_SIZE: u32 = 1000;
const TREASURY: &str = "5S7WgdAXVK7mh8REvXfk9LdHs3Xqu9B2E9zzY8e4LE8Gg2ZX";

#[subxt::subxt(
    runtime_metadata_path = "sherpax_metadata.scale",
    generated_type_derives = "Clone, Debug"
)]
pub mod sherpax {}

/// CLI for submitting.
#[derive(Debug, StructOpt)]
struct Opts {
    #[structopt(subcommand)]
    command: Command,
}

#[derive(Debug, StructOpt)]
enum Command {
    /// Check Balance.
    #[structopt(name = "check-balance")]
    CheckBalance {
        #[structopt(flatten)]
        extrinsic_opts: ExtrinsicOpts,
    },
}
/// Arguments required for creating and sending an extrinsic to a sherpax node
#[derive(Clone, Debug, StructOpt)]
pub(crate) struct ExtrinsicOpts {
    /// Websockets url of a sherpax node
    #[structopt(name = "url", long, default_value = "ws://localhost:9977")]
    url: String,
    /// The specified block number.
    #[structopt(long)]
    block_number: Option<u32>,
    /// Enable print the detail info(every 10000 blocks)
    #[structopt(long)]
    print_details: bool
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct BalanceInfo {
    free: u128,
    locked: u128,
    reserved: u128,
    transferable: u128,
    misc_frozen: u128,
    fee_frozen: u128,
    accounts: u32,
    elapsed: u64,
    block: u32,
    treasury: String,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct TotalInfo {
    origin: BalanceInfo,
    transferable_exclude_treasury: u128,
    treasury_balance: u128,
    total_supply: u128,
}

impl TotalInfo {
    pub fn new() -> Self {
        Self {
            origin: BalanceInfo {
                free: 0,
                locked: 0,
                reserved: 0,
                transferable: 0,
                misc_frozen: 0,
                fee_frozen: 0,
                accounts: 0,
                elapsed: 0,
                block: 0,
                treasury: TREASURY.to_string(),
            },
            transferable_exclude_treasury: 0,
            treasury_balance: 0,
            total_supply: 0
        }
    }

    pub fn total_balance(&self) -> u128 {
        self.origin
            .free
            .saturating_add(self.origin.reserved)
    }

    pub fn total_transferable_exclude_treasury(&self) -> u128 {
        self
            .origin
            .transferable
            .saturating_sub(self.treasury_balance)
    }

    pub fn sanitize(&mut self) {
        self.transferable_exclude_treasury = self.total_transferable_exclude_treasury();
        self.total_supply = self.total_balance();
    }
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let now = std::time::Instant::now();

    let opts = Opts::from_args();
    let Command::CheckBalance {
        extrinsic_opts,
    } = opts.command;

    let api = ClientBuilder::new()
        .set_url(extrinsic_opts.url)
        .set_page_size(MAX_PAG_SIZE)
        .build()
        .await?
        .to_runtime_api::<sherpax::RuntimeApi<sherpax::DefaultConfig>>();

    let block_number = {
        if let Some(number) = extrinsic_opts.block_number {
            number
        } else {
            api
                .client
                .rpc()
                .block(None)
                .await?
                .expect("Failed to fetch the latest block")
                .block
                .header
                .number
        }
    };

    let block_hash = api
        .client
        .rpc()
        .block_hash(Some(BlockNumber::from(block_number)))
        .await?;

    let mut total_info = TotalInfo::new();
    total_info.origin.block = block_number;

    let treasury_account = AccountId32::from_ss58check(TREASURY)
        .expect("Failed to parse treasury account");

    let treasury_free = api
        .storage()
        .system()
        .account(treasury_account, block_hash)
        .await?
        .data
        .free;

    total_info.treasury_balance = treasury_free;

    let mut iter = api.storage().system().account_iter(block_hash).await?;

    while let Some((_, account)) = iter.next().await? {
        total_info.origin.accounts += 1;
        total_info.origin.free += account.data.free;
        total_info.origin.reserved += account.data.reserved;
        total_info.origin.misc_frozen += account.data.misc_frozen;
        total_info.origin.fee_frozen += account.data.fee_frozen;

        let locked = account.data.misc_frozen.max(account.data.fee_frozen);
        total_info.origin.locked += locked;
        total_info.origin.transferable += account.data.free - locked;

        if extrinsic_opts.print_details && total_info.origin.accounts % 10000 == 0 {
            total_info.origin.elapsed = now.elapsed().as_secs();
            println!("{}", serde_json::to_string(&total_info)?);
        }
    }

    total_info.origin.elapsed = now.elapsed().as_secs();

    total_info.sanitize();

    if extrinsic_opts.print_details {
        println!("{}", serde_json::to_string(&total_info)?);

        assert_eq!(
            total_info.total_balance(),
            total_info
                .treasury_balance
                .saturating_add(total_info.transferable_exclude_treasury)
                .saturating_add(total_info.origin.locked)
                .saturating_add(total_info.origin.reserved)
        );
    }

    let json_format = json!({
            "treasury_balance": format!("{}", total_info.treasury_balance),
            "transferable_exclude_treasury": format!("{}", total_info.transferable_exclude_treasury),
            "locked": format!("{}", total_info.origin.locked),
            "reserved": format!("{}", total_info.origin.reserved),
            "block_number": total_info.origin.block
        });

    println!("{}", json_format);

    Ok(())
}
