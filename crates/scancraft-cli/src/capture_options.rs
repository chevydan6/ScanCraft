//! Shared optional illumination-capture controls. Device gain bounds remain authoritative.
pub fn parse(args: &[String]) -> Result<(u32, Option<i32>), String> {
    let mut fps = None;
    let mut gain = None;
    let (pairs, remainder) = args.as_chunks::<2>();
    for pair in pairs {
        match pair[0].as_str() {
            "--fps" if fps.is_none() => {
                fps = Some(pair[1].parse().map_err(|_| "invalid FPS")?);
            }
            "--gain" if gain.is_none() => {
                gain = Some(pair[1].parse().map_err(|_| "invalid raw gain")?);
            }
            _ => return Err("expected unique --fps FPS and/or --gain RAW_GAIN".into()),
        }
    }
    if !remainder.is_empty() {
        return Err("capture option requires a value".into());
    }
    Ok((fps.unwrap_or(50), gain))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options(values: &[&str]) -> Result<(u32, Option<i32>), String> {
        parse(&values.iter().map(|s| s.to_string()).collect::<Vec<_>>())
    }
    #[test]
    fn retains_defaults_and_accepts_combined_controls_in_either_order() {
        assert_eq!(options(&[]).unwrap(), (50, None));
        assert_eq!(options(&["--fps", "30"]).unwrap(), (30, None));
        assert_eq!(options(&["--gain", "8000"]).unwrap(), (50, Some(8000)));
        for args in [
            ["--fps", "30", "--gain", "8000"],
            ["--gain", "8000", "--fps", "30"],
        ] {
            assert_eq!(options(&args).unwrap(), (30, Some(8000)));
        }
    }
    #[test]
    fn rejects_incomplete_ambiguous_and_unknown_control_requests() {
        for args in [
            vec!["--gain"],
            vec!["--fps"],
            vec!["--fps", "30", "--gain"],
            vec!["--fps", "30", "--fps", "40"],
            vec!["--gain", "8000", "--gain", "9000"],
            vec!["--energy", "8000"],
            vec!["--gain", "invalid"],
            vec!["--gain", "2147483648"],
            vec!["--fps", "-1"],
        ] {
            assert!(options(&args).is_err(), "{args:?}");
        }
    }
}
