use crate::args::Args;
use crate::format::pattern::Pattern;
use crate::format::{CrateDetectionStatus, FormatError};

use cargo::util::errors::CliError;
use colored::{ColoredString, Colorize};
use geiger::IncludeTests;
use petgraph::{Direction, EdgeDirection};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Prefix {
    Depth,
    Indent,
    None,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Ascii,
    Json,
    GitHubMarkdown,
    Ratio,
    Utf8,
}

impl Default for OutputFormat {
    fn default() -> Self {
        OutputFormat::Utf8
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = OutputFormatParseError;
    fn from_str(s: &str) -> Result<Self, OutputFormatParseError> {
        match s {
            "Ascii" => Ok(Self::Ascii),
            "Json" => Ok(Self::Json),
            "GitHubMarkdown" => Ok(Self::GitHubMarkdown),
            "Ratio" => Ok(Self::Ratio),
            "Utf8" => Ok(Self::Utf8),
            _ => Err(OutputFormatParseError),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OutputFormatParseError;
impl std::fmt::Display for OutputFormatParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "matching output format not found")
    }
}
impl std::error::Error for OutputFormatParseError {}

#[derive(Debug, Eq, PartialEq)]
pub struct PrintConfig {
    /// Don't truncate dependencies that have already been displayed.
    pub all: bool,

    pub allow_partial_results: bool,
    pub direction: EdgeDirection,

    // Is anyone using this? This is a carry-over from cargo-tree.
    // TODO: Open a github issue to discuss deprecation.
    pub format: Pattern,

    pub include_tests: IncludeTests,
    pub prefix: Prefix,
    pub output_format: OutputFormat,
}

impl PrintConfig {
    pub fn new(args: &Args) -> Result<Self, CliError> {
        // TODO: Add command line flag for this and make it default to false?
        let allow_partial_results = true;

        let direction = match args.invert {
            true => EdgeDirection::Incoming,
            false => EdgeDirection::Outgoing,
        };

        let format = Pattern::try_build(&args.format).map_err(|e| {
            CliError::new(
                (FormatError {
                    message: e.to_string(),
                })
                .into(),
                1,
            )
        })?;

        let include_tests = match args.include_tests {
            true => IncludeTests::Yes,
            false => IncludeTests::No,
        };

        let prefix = match (args.prefix_depth, args.no_indent) {
            (true, _) => Prefix::Depth,
            (false, true) => Prefix::None,
            (false, false) => Prefix::Indent,
        };

        Ok(PrintConfig {
            all: args.all,
            allow_partial_results,
            direction,
            format,
            include_tests,
            output_format: args.output_format,
            prefix,
        })
    }
}

impl Default for PrintConfig {
    fn default() -> Self {
        PrintConfig {
            all: false,
            allow_partial_results: false,
            direction: Direction::Outgoing,
            format: Pattern::try_build("p").unwrap(),
            include_tests: IncludeTests::Yes,
            prefix: Prefix::Depth,
            output_format: Default::default(),
        }
    }
}

pub fn colorize(
    crate_detection_status: &CrateDetectionStatus,
    output_format: OutputFormat,
    string: String,
) -> ColoredString {
    match output_format {
        OutputFormat::GitHubMarkdown => ColoredString::from(string.as_str()),
        _ => match crate_detection_status {
            CrateDetectionStatus::NoneDetectedForbidsUnsafe => string.green(),
            CrateDetectionStatus::NoneDetectedAllowsUnsafe => string.normal(),
            CrateDetectionStatus::UnsafeDetected => string.red().bold(),
        },
    }
}

#[cfg(test)]
mod print_config_tests {
    use super::*;

    use crate::format::pattern::Pattern;
    use crate::format::Chunk;

    use rstest::*;
    use std::str::FromStr;

    #[rstest(
        input_invert_bool,
        expected_edge_direction,
        case(true, EdgeDirection::Incoming),
        case(false, EdgeDirection::Outgoing)
    )]
    fn print_config_new_test_invert(
        input_invert_bool: bool,
        expected_edge_direction: EdgeDirection,
    ) {
        let args = Args {
            invert: input_invert_bool,
            ..Default::default()
        };

        let print_config_result = PrintConfig::new(&args);

        assert!(print_config_result.is_ok());
        assert_eq!(
            print_config_result.unwrap().direction,
            expected_edge_direction
        );
    }

    #[rstest(
        input_format_string,
        expected_format,
        case(String::from("{p}"), Pattern::new(vec![Chunk::Package])),
        case(String::from("{l}"), Pattern::new(vec![Chunk::License])),
        case(String::from("{r}"), Pattern::new(vec![Chunk::Repository])),
        case(String::from("Text"), Pattern::new(vec![Chunk::Raw(String::from("Text"))])),
        case(
            String::from("{p}-{l}-{r}-Text"),
            Pattern {
                chunks: vec ! [
                    Chunk::Package,
                    Chunk::Raw(String::from("-")),
                    Chunk::License,
                    Chunk::Raw(String::from("-")),
                    Chunk::Repository,
                    Chunk::Raw(String::from("-Text"))
                ]
            }
        )
    )]
    fn print_config_new_test_format(
        input_format_string: String,
        expected_format: Pattern,
    ) {
        let args = Args {
            format: input_format_string,
            ..Default::default()
        };

        let print_config_result = PrintConfig::new(&args);

        assert!(print_config_result.is_ok());
        assert_eq!(print_config_result.unwrap().format, expected_format);
    }

    #[rstest(
        input_include_tests_bool,
        expected_include_tests,
        case(true, IncludeTests::Yes),
        case(false, IncludeTests::No)
    )]
    fn print_config_new_test_include_tests(
        input_include_tests_bool: bool,
        expected_include_tests: IncludeTests,
    ) {
        let args = Args {
            include_tests: input_include_tests_bool,
            ..Default::default()
        };

        let print_config_result = PrintConfig::new(&args);

        assert!(print_config_result.is_ok());
        assert_eq!(
            print_config_result.unwrap().include_tests,
            expected_include_tests
        );
    }

    #[rstest(
        input_prefix_depth_bool,
        input_no_indent_bool,
        expected_output_prefix,
        case(true, false, Prefix::Depth,),
        case(true, false, Prefix::Depth,),
        case(false, true, Prefix::None,),
        case(false, false, Prefix::Indent,)
    )]
    fn print_config_new_test_prefix(
        input_prefix_depth_bool: bool,
        input_no_indent_bool: bool,
        expected_output_prefix: Prefix,
    ) {
        let args = Args {
            no_indent: input_no_indent_bool,
            prefix_depth: input_prefix_depth_bool,
            ..Default::default()
        };

        let print_config_result = PrintConfig::new(&args);

        assert!(print_config_result.is_ok());
        assert_eq!(print_config_result.unwrap().prefix, expected_output_prefix);
    }

    #[rstest(
        input_raw_str,
        expected_output_format_result,
        case("Ascii", Ok(OutputFormat::Ascii)),
        case("Json", Ok(OutputFormat::Json)),
        case("GitHubMarkdown", Ok(OutputFormat::GitHubMarkdown)),
        case("Utf8", Ok(OutputFormat::Utf8)),
        case("unknown_variant", Err(OutputFormatParseError))
    )]
    fn output_format_from_str_test(
        input_raw_str: &str,
        expected_output_format_result: Result<
            OutputFormat,
            OutputFormatParseError,
        >,
    ) {
        let output_format = OutputFormat::from_str(input_raw_str);
        assert_eq!(output_format, expected_output_format_result);
    }

    #[rstest(
        input_crate_detection_status,
        input_output_format,
        expected_colored_string,
        case(
            CrateDetectionStatus::NoneDetectedForbidsUnsafe,
            OutputFormat::Ascii,
            String::from("string_value").green()
        ),
        case(
            CrateDetectionStatus::NoneDetectedAllowsUnsafe,
            OutputFormat::Utf8,
            String::from("string_value").normal()
        ),
        case(
            CrateDetectionStatus::UnsafeDetected,
            OutputFormat::Ascii,
            String::from("string_value").red().bold()
        ),
        case(
            CrateDetectionStatus::NoneDetectedForbidsUnsafe,
            OutputFormat::GitHubMarkdown,
            ColoredString::from("string_value")
        ),
        case(
            CrateDetectionStatus::NoneDetectedAllowsUnsafe,
            OutputFormat::GitHubMarkdown,
            ColoredString::from("string_value")
        ),
        case(
            CrateDetectionStatus::UnsafeDetected,
            OutputFormat::GitHubMarkdown,
            ColoredString::from("string_value")
        )
    )]
    fn colorize_test(
        input_crate_detection_status: CrateDetectionStatus,
        input_output_format: OutputFormat,
        expected_colored_string: ColoredString,
    ) {
        let string_value = String::from("string_value");

        assert_eq!(
            colorize(
                &input_crate_detection_status,
                input_output_format,
                string_value
            ),
            expected_colored_string
        );
    }
}
