use clap::{App, Arg, ArgMatches, Values};
use failure::{format_err, Error};
use lucetc::{CpuFeatures, HeapSettings, OptLevel, SpecificFeatures, TargetCpu};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodegenOutput {
    Clif,
    Obj,
    SharedObj,
}

fn parse_humansized(desc: &str) -> Result<u64, Error> {
    use human_size::{Byte, ParsingError, Size, SpecificSize};
    match desc.parse::<Size>() {
        Ok(s) => {
            let bytes: SpecificSize<Byte> = s.into();
            Ok(bytes.value() as u64)
        }
        Err(ParsingError::MissingMultiple) => Ok(desc.parse::<u64>()?),
        Err(e) => Err(e)?,
    }
}

fn humansized(bytes: u64) -> String {
    use human_size::{Byte, Mebibyte, SpecificSize};
    let bytes = SpecificSize::new(bytes as f64, Byte).expect("bytes");
    let mb: SpecificSize<Mebibyte> = bytes.into();
    mb.to_string()
}

fn cpu_features_from_args(cpu: Option<&str>, features: Values) -> Result<CpuFeatures, Error> {
    if cpu.is_none() && features.len() == 0 {
        Ok(CpuFeatures::DetectCpuid)
    } else {
        let mut sfs: SpecificFeatures = match cpu {
            None => TargetCpu::Baseline,
            Some(s) => match s.to_lowercase().as_str() {
                "baseline" => TargetCpu::Baseline,
                "nehalem" => TargetCpu::Nehalem,
                "haswell" => TargetCpu::Haswell,
                "broadwell" => TargetCpu::Broadwell,
                "skylake" => TargetCpu::Skylake,
                "cannonlake" => TargetCpu::Cannonlake,
                "icelake" => TargetCpu::Icelake,
                "znver1" => TargetCpu::Znver1,
                _ => Err(format_err!("unsupported CPU: {}", s))?,
            },
        }
        .into();
        for f in features {
            let b = match f.chars().nth(0) {
                Some('+') => true,
                Some('-') => false,
                _ => unreachable!("invalid feature string despite passing validation: {}", f),
            };
            // the only valid starting characters are single-byte '+' and '-', so this indexing
            // ought not to fail
            match &f[1..] {
                "sse3" => sfs.has_sse3 = b,
                "ssse3" => sfs.has_ssse3 = b,
                "sse41" => sfs.has_sse41 = b,
                "sse42" => sfs.has_sse42 = b,
                "popcnt" => sfs.has_popcnt = b,
                "avx" => sfs.has_avx = b,
                "bmi1" => sfs.has_bmi1 = b,
                "bmi2" => sfs.has_bmi2 = b,
                "lzcnt" => sfs.has_lzcnt = b,
                _ => unreachable!("invalid feature string despite passing validation: {}", f),
            }
        }
        Ok(CpuFeatures::Specify(sfs))
    }
}

#[derive(Debug)]
pub struct Options {
    pub output: PathBuf,
    pub input: Vec<PathBuf>,
    pub codegen: CodegenOutput,
    pub binding_files: Vec<PathBuf>,
    pub builtins_path: Option<PathBuf>,
    pub min_reserved_size: Option<u64>,
    pub max_reserved_size: Option<u64>,
    pub reserved_size: Option<u64>,
    pub guard_size: Option<u64>,
    pub opt_level: OptLevel,
    pub cpu_features: CpuFeatures,
    pub keygen: bool,
    pub sign: bool,
    pub verify: bool,
    pub pk_path: Option<PathBuf>,
    pub sk_path: Option<PathBuf>,
    pub count_instructions: bool,
}

impl Options {
    pub fn from_args(m: &ArgMatches<'_>) -> Result<Self, Error> {
        let input: Vec<PathBuf> = m
            .values_of("input")
            .unwrap_or_default()
            .map(PathBuf::from)
            .collect();

        let output = PathBuf::from(m.value_of("output").unwrap_or("a.out"));

        let binding_files: Vec<PathBuf> = m
            .values_of("bindings")
            .unwrap_or_default()
            .map(PathBuf::from)
            .collect();

        let codegen = match m.value_of("emit") {
            None => CodegenOutput::SharedObj,
            Some("clif") => CodegenOutput::Clif,
            Some("obj") => CodegenOutput::Obj,
            Some("so") => CodegenOutput::SharedObj,
            Some(_) => panic!("unknown value for emit"),
        };

        let builtins_path = m.value_of("builtins").map(PathBuf::from);

        let min_reserved_size = if let Some(min_reserved_str) = m.value_of("min_reserved_size") {
            Some(parse_humansized(min_reserved_str)?)
        } else {
            None
        };

        let max_reserved_size = if let Some(max_reserved_str) = m.value_of("max_reserved_size") {
            Some(parse_humansized(max_reserved_str)?)
        } else {
            None
        };

        let reserved_size = if let Some(reserved_str) = m.value_of("reserved_size") {
            Some(parse_humansized(reserved_str)?)
        } else {
            None
        };

        let guard_size = if let Some(guard_str) = m.value_of("guard_size") {
            Some(parse_humansized(guard_str)?)
        } else {
            None
        };

        let opt_level = match m.value_of("opt_level") {
            None => OptLevel::default(),
            Some("0") => OptLevel::None,
            Some("1") => OptLevel::Standard,
            Some("2") | Some("fast") => OptLevel::Fast,
            Some(_) => panic!("unknown value for opt-level"),
        };
        let cpu_features = cpu_features_from_args(
            m.value_of("target-cpu"),
            m.values_of("target-feature").unwrap_or_default(),
        )?;

        let keygen = m.is_present("keygen");
        let sign = m.is_present("sign");
        let verify = m.is_present("verify");
        let sk_path = m.value_of("sk_path").map(PathBuf::from);
        let pk_path = m.value_of("pk_path").map(PathBuf::from);
        let count_instructions = m.is_present("count_instructions");

        Ok(Options {
            output,
            input,
            codegen,
            binding_files,
            builtins_path,
            min_reserved_size,
            max_reserved_size,
            reserved_size,
            guard_size,
            opt_level,
            cpu_features,
            keygen,
            sign,
            verify,
            sk_path,
            pk_path,
            count_instructions,
        })
    }
    pub fn get() -> Result<Self, Error> {
        let m = App::new("lucetc")
            .arg(
                Arg::with_name("precious")
                    .long("--precious")
                    .takes_value(true)
                    .help("directory to keep intermediate build artifacts in"),
            )
            .arg(
                Arg::with_name("emit")
                    .long("emit")
                    .takes_value(true)
                    .possible_values(&["obj", "so", "clif"])
                    .help("type of code to generate (default: so)"),
            )
            .arg(
                Arg::with_name("output")
                    .short("o")
                    .long("output")
                    .takes_value(true)
                    .multiple(false)
                    .help("output destination, defaults to a.out if unspecified"),
            )
            .arg(
                Arg::with_name("target-cpu")
                    .long("--target-cpu")
                    .takes_value(true)
                    .multiple(false)
                    .number_of_values(1)
                    .possible_values(&[
                        "baseline",
                        "nehalem",
                        "haswell",
                        "broadwell",
                        "skylake",
                        "cannonlake",
                        "icelake",
                        "znver1",
                    ])
                    .help("Generate code for a particular type of CPU.")
                    .long_help(
"Generate code for a particular type of CPU.

If neither `--target-cpu` nor `--target-feature` is provided, `lucetc`
will automatically detect and use the features available on the host CPU.

"
                    )
            )
            .arg(
                Arg::with_name("target-feature")
                    .long("--target-feature")
                    .takes_value(true)
                    .multiple(true)
                    .use_delimiter(true)
                    .possible_values(&[
                        "+sse3", "-sse3",
                        "+ssse3", "-ssse3",
                        "+sse41", "-sse41",
                        "+sse42", "-sse42",
                        "+popcnt", "-popcnt",
                        "+avx", "-avx",
                        "+bmi1", "-bmi1",
                        "+bmi2", "-bmi2",
                        "+lzcnt", "-lzcnt",
                    ])
                    .help("Enable (+) or disable (-) specific CPU features.")
                    .long_help(
"Enable (+) or disable (-) specific CPU features.

If neither `--target-cpu` nor `--target-feature` is provided, `lucetc`
will automatically detect and use the features available on the host CPU.

This option is additive with, but takes precedence over `--target-cpu`.
For example, `--target-cpu=haswell --target-feature=-avx` will disable
AVX, but leave all other default Haswell features enabled.

Multiple `--target-feature` groups may be specified, with precedence
increasing from left to right. For example, these arguments will enable
SSE3 but not AVX:

    --target-feature=+sse3,+avx --target-feature=-avx

"
                    )
            )
            .arg(
                Arg::with_name("bindings")
                    .long("--bindings")
                    .takes_value(true)
                    .multiple(true)
                    .number_of_values(1)
                    .help("path to bindings json file"),
            )
            .arg(
                Arg::with_name("min_reserved_size")
                    .long("--min-reserved-size")
                    .takes_value(true)
                    .multiple(false)
                    .help(&format!(
                        "minimum size of usable linear memory region. must be multiple of 4k. default: {}",
                        humansized(HeapSettings::default().min_reserved_size)
                    )),
            )
            .arg(
                Arg::with_name("max_reserved_size")
                    .long("--max-reserved-size")
                    .takes_value(true)
                    .multiple(false)
                    .help("maximum size of usable linear memory region. must be multiple of 4k. default: 4 GiB"),
            )
            .arg(
                Arg::with_name("reserved_size")
                    .long("--reserved-size")
                    .takes_value(true)
                    .multiple(false)
                    .help("exact size of usable linear memory region, overriding --{min,max}-reserved-size. must be multiple of 4k"),
            )
            .arg(
                Arg::with_name("guard_size")
                    .long("--guard-size")
                    .takes_value(true)
                    .multiple(false)
                    .help(&format!(
                        "size of linear memory guard. must be multiple of 4k. default: {}",
                        humansized(HeapSettings::default().guard_size)
                    )),
            )
            .arg(
                Arg::with_name("builtins")
                    .long("--builtins")
                    .takes_value(true)
                    .help("builtins file"),
            )
            .arg(
                Arg::with_name("input")
                    .multiple(false)
                    .required(false)
                    .help("input file"),
            )
            .arg(
                Arg::with_name("opt_level")
                    .long("--opt-level")
                    .takes_value(true)
                    .possible_values(&["0", "1", "2", "fast"])
                    .help("optimization level (default: '1')"),
            )
            .arg(
                Arg::with_name("keygen")
                    .long("--signature-keygen")
                    .takes_value(false)
                    .help("Create a new key pair")
            )
            .arg(
                Arg::with_name("verify")
                     .long("--signature-verify")
                     .takes_value(false)
                     .help("Verify the signature of the source file")
            )
            .arg(
                Arg::with_name("sign")
                     .long("--signature-create")
                     .takes_value(false)
                     .help("Sign the object file")
            )
            .arg(
                Arg::with_name("pk_path")
                     .long("--signature-pk")
                     .takes_value(true)
                     .help("Path to the public key to verify the source code signature")
            )
            .arg(
                Arg::with_name("sk_path")
                     .long("--signature-sk")
                     .takes_value(true)
                     .help("Path to the secret key to sign the object file. The file can be prefixed with \"raw:\" in order to store a raw, unencrypted secret key")
            )
            .arg(
                Arg::with_name("count_instructions")
                    .long("--count-instructions")
                    .takes_value(false)
                    .help("Instrument the produced binary to count the number of wasm operations the translated program executes")
            )
            .get_matches();

        Self::from_args(&m)
    }
}
