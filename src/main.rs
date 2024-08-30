use clap::Parser;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// The input bed file
    #[clap(short, long)]
    input: String,

    /// The output bedgraph file
    /// If not provided, the output will be printed to stdout
    #[clap(short, long)]
    output: Option<String>,

    /// The index of the column containing the value to graph which can be 'score' or the column index (min 0).
    /// The value must be a number
    #[clap(short, long, default_value = "0")]
    value_column: String,
}

#[derive(Debug)]
struct BedRecord {
    chrom: String,
    start: u32,
    end: u32,
    name: String,
    score: f64,
    values: Vec<String>,
}

struct BedParser {
    input_file: BufReader<File>,
}

impl BedParser {
    fn new(input_file: &str) -> Self {
        let file = File::open(input_file).expect("Could not open file");
        let reader = BufReader::new(file);
        BedParser { input_file: reader }
    }
}

impl Iterator for BedParser {
    type Item = BedRecord;

    fn next(&mut self) -> Option<Self::Item> {
        let mut line = String::new();
        self.input_file
            .read_line(&mut line)
            .expect("Could not read line");
        if line.is_empty() {
            return None;
        }
        let fields: Vec<&str> = line.trim().split('\t').collect();
        let chrom = fields[0].to_string();
        let start = fields[1].parse::<u32>().expect("Could not parse start");
        let end = fields[2].parse::<u32>().expect("Could not parse end");
        let name = fields[3].to_string();
        let score = fields[4].parse::<f64>().unwrap_or(0.0);
        let values = fields[5..].iter().map(|x| x.to_string()).collect();
        Some(BedRecord {
            chrom,
            start,
            end,
            name,
            values,
            score,
        })
    }
}

struct BedGraphRecord {
    chrom: String,
    start: u32,
    end: u32,
    value: f64,
}

struct BedGraphWriter<W: Write> {
    writer: W,
}

impl<W: Write> BedGraphWriter<W> {
    fn new(writer: W) -> Self {
        let mut writer = writer;
        writer
            .write_all(b"track type=bedGraph\n")
            .expect("Could not write header");
        BedGraphWriter { writer }
    }

    fn write(&mut self, record: &BedGraphRecord) -> std::io::Result<()> {
        writeln!(
            self.writer,
            "{}\t{}\t{}\t{}",
            record.chrom, record.start, record.end, record.value
        )
    }
}

fn create_bedgraph_writer(
    output_file: Option<&str>,
) -> std::io::Result<BedGraphWriter<Box<dyn Write>>> {
    let writer: Box<dyn Write> = match output_file {
        Some(filename) => Box::new(BufWriter::new(File::create(filename)?)),
        None => Box::new(BufWriter::new(std::io::stdout())),
    };
    Ok(BedGraphWriter::new(writer))
}

fn main() {
    let args = Cli::parse();

    let parser = BedParser::new(&args.input);

    let mut writer =
        create_bedgraph_writer(args.output.as_deref()).expect("Could not create writer");

    let index_to_parse = match args.value_column.as_str() {
        "score" => -1,
        _ => args
            .value_column
            .parse::<i32>()
            .expect("Could not parse column index"),
    };

    let mut has_parsed_first_line = false;

    for record in parser {
        if !has_parsed_first_line {
            if index_to_parse > 0 && index_to_parse >= record.values.len() as i32 {
                eprintln!(
                    "Could not find column index {} in record. Remember that the index is 0-based and the first value is after the score column",
                    index_to_parse
                );
                break;
            }
        }

        let value = match index_to_parse {
            -1 => record.score,
            _ => record.values[index_to_parse as usize]
                .parse::<f64>()
                .expect("Could not parse value"),
        };
        let bg_record = BedGraphRecord {
            chrom: record.chrom,
            start: record.start,
            end: record.end,
            value,
        };
        writer.write(&bg_record).expect("Could not write record");

        if !has_parsed_first_line {
            has_parsed_first_line = true;
        }
    }

    // the value_column
}
