
use fst::SetBuilder;

use std::fs::File;
use std::io::{
    BufRead,
    BufReader,
    BufWriter,
};
use std::time::Instant;

use std::path::PathBuf;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "make_fst", about = "Create a fst file from a word list, this can be useful for faster loading")]
struct Opt {
    /// The input list. One word per line
    #[structopt(short = "i", long = "input-list", parse(from_os_str))]
    list_file: PathBuf,
    
    /// The output for in which store the compressed dictionary
    #[structopt(short = "o", long = "output-fst", parse(from_os_str))]
    fst_file: PathBuf,
}

fn main() {
    simple_logger::SimpleLogger::new().with_level(log::LevelFilter::Info).init().unwrap();
    
    let opts = Opt::from_args();
    
    let Opt { list_file, fst_file } = opts;
    
    let start = Instant::now();
    let file = BufReader::new(File::open(list_file).expect("opening the words list file"));
    let mut words = file.lines().map(|l|
        l.expect("reading line from word list").trim().to_lowercase()
    ).collect::<Vec<_>>();
    log::info!("words loaded in {:?}", Instant::now() - start);
    
    let start = Instant::now();
    words.sort_unstable();
    log::info!("words sorted in {:?}", Instant::now() - start);
    
    let start = Instant::now();
    let wtr = BufWriter::new(File::create(fst_file).expect("create the words fst file"));
    let mut build = SetBuilder::new(wtr).expect("builder wrting to fst file");
    build.extend_iter(words).unwrap();
    build.finish().unwrap();
    log::info!("dictionary written in {:?}", Instant::now() - start);
}
