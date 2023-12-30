use solana_devtools_signers::ConcreteSigner;
use solana_sdk::signer::Signer;

fn main() {
    let signer = ConcreteSigner::from_uri("usb://ledger?key=0").unwrap();
    println!("{}", signer.pubkey().to_string());
}
