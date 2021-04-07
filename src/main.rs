mod cli;
mod config;
mod repository {
    pub(crate) mod remote;
}

use tokio;

#[tokio::main]
async fn main() {
    if let Err(ref errs) = cli::run().await {
        let mut errs = errs.iter();
        match errs.next() {
            None => (),
            Some(err) => {
                eprintln!("{}", err);
                errs.for_each(|err| eprintln!(" ↳ {}", err));
            }
        }
    }
}
