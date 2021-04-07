use tokio;

mod cache;
mod cli;
mod config;
mod local {
    pub(crate) mod model;
    pub(crate) mod repository;
}
mod remote {
    pub(crate) mod model;
    pub(crate) mod repository;
}

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
