//use regex::Regex;
use helper::date::*;
use onig::*;
use structs::Color;
use structs::NumberingFormat;
use thousands::Separable;

const DATE_FORMAT_REPLACEMENTS: &[(&str, &str)] = &[
    // first remove escapes related to non-format characters
    ("\\", ""),
    // 12-hour suffix
    ("am/pm", "%P"),
    // Era - no rust equivalent
    ("ggge", "%Y"),
    // 4-digit year
    ("e", "%Y"),
    ("yyyy", "%Y"),
    // 2-digit year
    ("yy", "%y"),
    // first letter of month - no rust equivalent
    ("mmmmm", "%b"),
    // full month name
    ("mmmm", "%B"),
    // short month name
    ("mmm", "%b"),
    // mm is minutes if time, but can also be month w/leading zero
    // so we try to identify times be the inclusion of a : separator in the mask
    // It isn't perfect, but the best way I know how
    (":mm", ":%M"),
    ("mm:", "%M:"),
    // month leading zero (first)
    ("mm", "MM"),
    // month no leading zero
    ("m", "%-m"),
    // month leading zero (second)
    ("MM", "%m"),
    // full day of week name
    ("dddd", "%A"),
    // short day of week name
    ("ddd", "%a"),
    // days leading zero (first)
    ("dd", "D"),
    // days no leading zero
    ("d", "%-d"),
    // days leading zero (second)
    ("D", "%d"),
    // seconds
    ("ss", "%S"),
    // fractional seconds - no rust equivalent
    (".s", ""),
];

const DATE_FORMAT_REPLACEMENTS_24: &[(&str, &str)] = &[("hh", "%H"), ("h", "%-H")];

const DATE_FORMAT_REPLACEMENTS_12: &[(&str, &str)] = &[("hh", "%I"), ("h", "%-I")];

pub fn to_formatted_string<S: Into<String>>(value: S, format: S) -> String {
    let mut value = value.into();
    let mut format = format.into();

    // is empty
    if value.is_empty() {
        return value;
    }

    // is numeric
    match &value.parse::<f64>() {
        Ok(_) => {}
        Err(_) => return value,
    }

    // convert value
    if &format == NumberingFormat::FORMAT_GENERAL {
        return value.parse::<f64>().unwrap().to_string();
    }
    if &format == NumberingFormat::FORMAT_TEXT {
        return value;
    }

    // Convert any other escaped characters to quoted strings, e.g. (\T to "T")
    let re =
        Regex::new(r#"(\\\(((.)(?!((AM\/PM)|(A\/P)))|([^ ])))(?=(?:[^"]|"[^"]*")*$)"#).unwrap();
    format = re.replace_all(&format, r#""$0""#);

    // Get the sections, there can be up to four sections, separated with a semi-colon (but only if not a quoted literal)
    let re = Regex::new(r#"(;)(?=(?:[^"]|"[^"]*")*$)"#).unwrap();
    let sections: Vec<&str> = re.split(&format).collect();

    let (_, split_format, split_value) = split_format(sections, &value.parse::<f64>().unwrap());
    format = split_format;
    value = split_value;

    // In Excel formats, "_" is used to add spacing,
    //    The following character indicates the size of the spacing, which we can't do in HTML, so we just use a standard space
    let re = Regex::new("_.").unwrap();
    format = re.replace_all(&format, " ");

    // Let's begin inspecting the format and converting the value to a formatted string

    //  Check for date/time characters (not inside quotes)
    let re = Regex::new(r#"(\[\$[A-Z]*-[0-9A-F]*\])*[hmsdy](?=(?:[^"]|"[^"]*")*$)"#).unwrap();
    let re2 = Regex::new("%$").unwrap();
    if re.find(&format).is_some() {
        // datetime format
        value = format_as_date(&value.parse::<f64>().unwrap(), &format);
    } else if &format.starts_with('"') == &true && &format.ends_with('"') == &true {
        let conv_format = format.trim_matches('"').parse::<f64>().unwrap();
        value = conv_format.to_string();
    } else if re2.find(&format).is_some() {
        // % number format
        value = format_as_percentage(&value.parse::<f64>().unwrap(), &format);
    } else {
        value = format_as_number(&value.parse::<f64>().unwrap(), &format);
    }
    value.trim().to_string()
}

fn format_as_percentage(value: &f64, format: &str) -> String {
    let mut value = value.to_string();
    let mut format = format.to_string();
    format = format.replace('%', "");
    let blocks: Vec<&str> = format.split('.').collect();
    let len = match blocks.get(1) {
        Some(v) => v.len(),
        None => 0,
    };
    value = format!(
        "{:0width$.len$}%",
        (100f64 * &value.parse::<f64>().unwrap()).round(),
        width = 1,
        len = len
    );
    value
}

fn split_format(sections: Vec<&str>, value: &f64) -> (String, String, String) {
    let mut converted_sections: Vec<String> = Vec::new();

    // Extract the relevant section depending on whether number is positive, negative, or zero?
    // Text not supported yet.
    // Here is how the sections apply to various values in Excel:
    //   1 section:   [POSITIVE/NEGATIVE/ZERO/TEXT]
    //   2 sections:  [POSITIVE/ZERO/TEXT] [NEGATIVE]
    //   3 sections:  [POSITIVE/TEXT] [NEGATIVE] [ZERO]
    //   4 sections:  [POSITIVE] [NEGATIVE] [ZERO] [TEXT]
    let cnt: usize = sections.len();
    let color_regex: String = format!("{}{}{}", "\\[(", Color::NAMED_COLORS.join("|"), ")\\]");
    let cond_regex = r#"\[(>|>=|<|<=|=|<>)([+-]?\d+([.]\d+)?)\]"#;
    let color_re = Regex::new(&color_regex).unwrap();
    let cond_re = Regex::new(cond_regex).unwrap();

    let mut colors = [
        String::from(""),
        String::from(""),
        String::from(""),
        String::from(""),
        String::from(""),
    ];
    let mut condops = [
        String::from(""),
        String::from(""),
        String::from(""),
        String::from(""),
        String::from(""),
    ];
    let mut condvals = [
        String::from("0"),
        String::from("0"),
        String::from("0"),
        String::from("0"),
        String::from("0"),
    ];
    sections.into_iter().enumerate().for_each(|(idx, section)| {
        let mut converted_section = section.to_string();
        if color_re.find(section).is_some() {
            let mut item: Vec<String> = Vec::new();
            for ite in color_re.captures(section).unwrap().iter() {
                item.push(ite.unwrap().to_string());
            }
            let _ = std::mem::replace(&mut colors[idx], item.get(0).unwrap().to_string());
            converted_section = color_re.replace_all(section, "");
        }
        if cond_regex.contains(section) {
            let mut item: Vec<String> = Vec::new();
            for ite in cond_re.captures(section).unwrap().iter() {
                item.push(ite.unwrap().to_string());
            }
            let _ = std::mem::replace(&mut condops[idx], item.get(1).unwrap().to_string());
            let _ = std::mem::replace(&mut condvals[idx], item.get(2).unwrap().to_string());
            converted_section = cond_re.replace_all(section, "");
        }
        converted_sections.insert(idx, converted_section);
     });

    let mut color = colors[0].clone();
    let mut format: &str = &converted_sections[0];
    let mut absval = *value;
    match cnt {
        2 => {
            absval = absval.abs();
            let condval_one = &condvals[0].parse::<f64>().unwrap();
            if !split_format_compare(value, &condops[0], condval_one, ">=", &0f64) {
                color = colors[1].clone();
                format = &converted_sections[1];
            }
        }
        3 | 4 => {
            absval = absval.abs();
            let condval_one = &condvals[0].parse::<f64>().unwrap();
            let condval_two = &condvals[1].parse::<f64>().unwrap();
            if !split_format_compare(value, &condops[0], condval_one, ">", &0f64) {
                if split_format_compare(value, &condops[1], condval_two, "<", &0f64) {
                    color = colors[1].clone();
                    format = &converted_sections[1];
                } else {
                    color = colors[2].clone();
                    format = &converted_sections[2];
                }
            }
        }
        _ => {}
    }
    (color, format.into(), absval.to_string())
}

fn split_format_compare(value: &f64, cond: &str, val: &f64, dfcond: &str, dfval: &f64) -> bool {
    let mut check_cond = cond;
    let mut check_val = val;
    if cond.is_empty() {
        check_cond = dfcond;
        check_val = dfval;
    }
    match check_cond {
        ">" => return value > check_val,
        "<" => return value < check_val,
        "<=" => return value <= check_val,
        "<>" => return value != check_val,
        "=" => return value == check_val,
        _ => {}
    }
    value >= check_val
}

fn format_as_date(value: &f64, format: &str) -> String {
    let value = value;
    let mut format = format.to_string();

    // strip off first part containing e.g. [$-F800] or [$USD-409]
    // general syntax: [$<Currency string>-<language info>]
    // language info is in hexadecimal
    // strip off chinese part like [DBNum1][$-804]
    let re = Regex::new(r#"^(\[[0-9A-Za-z]*\])*(\[\$[A-Z]*-[0-9A-F]*\])"#).unwrap();
    format = re.replace_all(&format, r#""#);

    // OpenOffice.org uses upper-case number formats, e.g. 'YYYY', convert to lower-case;
    //    but we don't want to change any quoted strings
    let re = Regex::new(r#"(?:^|")([^"]*)(?:$|")"#).unwrap();
    format = re.replace_all(&format, |caps: &Captures| {
        let caps_string: String = caps.at(0).unwrap().parse().unwrap();
        caps_string.to_lowercase()
    });

    // Only process the non-quoted blocks for date format characters
    let blocks: Vec<&str> = format.split('"').collect();
    let mut converted_blocks: Vec<String> = Vec::new();
    let mut i = 0;
    for block in blocks {
        let mut block = block.to_string();
        if &i % &2 == 0 {
            for (before, after) in DATE_FORMAT_REPLACEMENTS {
                block = block.replace(before, after);
            }
            if block.find("%P") == None {
                // 24-hour time format
                // when [h]:mm format, the [h] should replace to the hours of the value * 24
                if block.find("[h]") != None {
                    let hours = value * &24f64;
                    block = block.replace("[h]", hours.to_string().as_str());
                    converted_blocks.push(block);
                    continue;
                }
                for (before, after) in DATE_FORMAT_REPLACEMENTS_24 {
                    block = block.replace(before, after);
                }
            } else {
                for (before, after) in DATE_FORMAT_REPLACEMENTS_12 {
                    block = block.replace(before, after);
                }
            }
        }
        converted_blocks.push(block);
        i += 1;
    }
    format = converted_blocks.join(r#""#);

    // escape any quoted characters so that DateTime format() will render them correctly
    let re = Regex::new(r#""(.*)""#).unwrap();
    format = re.replace_all(&format, |caps: &Captures| {
        let caps_string: String = caps.at(0).unwrap().parse().unwrap();
        caps_string.to_lowercase()
    });

    let date_obj = excel_to_date_time_object(value, None);
    date_obj.format(&format).to_string()
}

fn format_as_number(value: &f64, format: &str) -> String {
    let mut value = value.to_string();

    // The "_" in this string has already been stripped out,
    // so this test is never true. Furthermore, testing
    // on Excel shows this format uses Euro symbol, not "EUR".
    //if ($format === self::FORMAT_CURRENCY_EUR_SIMPLE) {
    //    return 'EUR ' . sprintf('%1.2f', $value);
    //}

    // Some non-number strings are quoted, so we'll get rid of the quotes, likewise any positional * symbols
    let mut format = format.replace('"', "").replace('*', "");

    // Find out if we need thousands separator
    // This is indicated by a comma enclosed by a digit placeholder:
    //        #,#   or   0,0
    let re = Regex::new(r#"(#,#|0,0)"#).unwrap();
    let converted_format_clone = format.clone();
    let use_thousands = re.find(&converted_format_clone).is_some();
    if &use_thousands == &true {
        format = Regex::new("0,0").unwrap().replace_all(&format, "00");
        format = Regex::new("#,#").unwrap().replace_all(&format, "##");
    }

    // Scale thousands, millions,...
    // This is indicated by a number of commas after a digit placeholder:
    //        #,   or    0.0,,
    let mut scale: f64 = 1f64; // same as no scale
    let re = Regex::new(r#"(#|0)(,+)"#).unwrap();
    if re.find(&format).is_some() {
        let mut matches: Vec<String> = Vec::new();
        for ite in re.captures(&format).unwrap().iter() {
            matches.push(ite.unwrap().to_string());
        }
        scale = 1000i32.pow(matches[2].len() as u32) as f64;

        // strip the commas
        format = Regex::new("0,+").unwrap().replace_all(&format, "0");
        format = Regex::new("#,+").unwrap().replace_all(&format, "#");
    }

    if Regex::new(r#"#?.*\?\/\?"#).unwrap().find(&format).is_some() {
        match &value.parse::<usize>() {
            Ok(_) => {}
            Err(_) => {
                value = format_as_fraction(&value.parse::<f64>().unwrap(), &format);
            }
        }
    } else {
        // Handle the number itself

        // scale number
        value = (value.parse::<f64>().unwrap() / scale).to_string();
        // Strip #
        format = Regex::new(r#"\#"#).unwrap().replace_all(&format, "0");
        // Remove \
        format = Regex::new(r#"\\"#).unwrap().replace_all(&format, "");
        // Remove locale code [$-###]
        format = Regex::new(r#"\[\$\-.*\]"#)
            .unwrap()
            .replace_all(&format, "");
        // Trim
        format = format.trim().to_string();

        let m = Regex::new(r#"\[[^\]]+\]"#)
            .unwrap()
            .replace_all(&format, "");
        let number_regex = r#"(0+)(\.?)(0*)"#;
        let re = Regex::new(number_regex).unwrap();
        if re.find(&m).is_some() {
            let mut item: Vec<String> = Vec::new();
            for ite in re.captures(&m).unwrap().iter() {
                item.push(ite.unwrap().to_string());
            }
            value =
                format_straight_numeric_value(&value, &format, &item, &use_thousands, number_regex);
        }
    }

    let re = Regex::new(r#"\$[^0-9]*"#).unwrap();
    if re.find(&format).is_some() {
        let mut item: Vec<String> = Vec::new();
        for ite in re.captures(&format).unwrap().iter() {
            item.push(ite.unwrap().to_string());
        }
        value = format!("{}{}", item.get(0).unwrap(), value);
        //    //  Currency or Accounting
        //    let currency_code = item.get(1).unwrap().to_string();
        //    value = Regex::new(r#"\[\$([^\]]*)\]"#).unwrap().replace_all(&value, currency_code.as_str()).to_string();
    }

    value
}

fn format_as_fraction(value: &f64, format: &str) -> String {
    let sign = if value < &0f64 { "-" } else { "" };

    let integer_part = value.abs().floor();
    let decimal_part = (value.abs() % 1f64)
        .to_string()
        .replace("0.", "")
        .parse::<f64>()
        .unwrap();
    let decimal_length = decimal_part.to_string().len();
    let decimal_divisor = 10i32
        .pow(decimal_length as u32)
        .to_string()
        .parse::<f64>()
        .unwrap();

    let gcd = gcd(&decimal_part, &decimal_divisor);

    let mut adjusted_decimal_part = &decimal_part / &gcd;
    let adjusted_decimal_divisor = &decimal_divisor / &gcd;

    let mut result = String::from("");
    match format.find('0') {
        Some(_) => {
            result = format!(
                "{}{} {}/{}",
                &sign, &integer_part, &adjusted_decimal_part, &adjusted_decimal_divisor
            );
        }
        None => match format.find('#') {
            Some(_) => {
                if integer_part == 0f64 {
                    result = format!(
                        "{}{}/{}",
                        &sign, &adjusted_decimal_part, &adjusted_decimal_divisor
                    );
                } else {
                    result = format!(
                        "{}{} {}/{}",
                        &sign, &integer_part, &adjusted_decimal_part, &adjusted_decimal_divisor
                    );
                }
            }
            None => {
                let check_format: String = format.chars().take(3).collect();
                if check_format == "? ?" {
                    let mut integer_part_str = integer_part.to_string();
                    if integer_part == 0f64 {
                        integer_part_str = String::from("");
                    }
                    result = format!(
                        "{}{} {}/{}",
                        &sign, &integer_part_str, &adjusted_decimal_part, &adjusted_decimal_divisor
                    );
                } else {
                    adjusted_decimal_part += &integer_part * &adjusted_decimal_divisor;
                    result = format!(
                        "{}{}/{}",
                        &sign, &adjusted_decimal_part, &adjusted_decimal_divisor
                    );
                }
            }
        },
    }
    result
}

fn format_straight_numeric_value(
    value: &str,
    _format: &str,
    matches: &Vec<String>,
    use_thousands: &bool,
    _number_regex: &str,
) -> String {
    let mut value = value.to_string();

    let right = matches.get(3).unwrap();

    // minimun width of formatted number (including dot)
    if use_thousands == &true {
        value = value.parse::<f64>().unwrap().separate_with_commas();
    }
    let blocks: Vec<&str> = value.split('.').collect();
    let left_value = blocks.get(0).unwrap().to_string();
    let mut right_value = match blocks.get(1) {
        Some(v) => v.to_string(),
        None => String::from("0"),
    };
    if right.is_empty() {
        return left_value;
    }
    if right.len() != right_value.len() {
        if right_value == "0" {
            right_value = right.to_string();
        } else if right.len() > right_value.len() {
            let pow = 10i32.pow(right.len() as u32);
            right_value = format!("{}", right_value.parse::<i32>().unwrap() * pow);
        } else {
            let mut right_value_conv: String =
                right_value.chars().skip(0).take(right.len()).collect();
            let ajst_str: String = right_value.chars().skip(right.len()).take(1).collect();
            let ajst_int = ajst_str.parse::<i32>().unwrap();
            if ajst_int > 4 {
                right_value_conv = (right_value_conv.parse::<i32>().unwrap() + 1).to_string();
            }
            right_value = right_value_conv;
        }
    }
    value = format!("{}.{}", left_value, right_value);
    value

    //    if use_thousands == &true {
    //        value = value.parse::<f64>().unwrap().separate_with_commas();
    //        dbg!(&value);
    //        value = Regex::new(&number_regex).unwrap().replace_all(&format, value.as_str());
    //        dbg!(&value);
    //    } else {
    //        if Regex::new(r#"[0#]E[+-]0"#).unwrap().find(&format).is_some() {
    //            // Scientific format
    //            value = value.parse::<f64>().unwrap().to_string();
    //        } else if Regex::new(r#"0([^\d\.]+)0"#).unwrap().find(&format).is_some() || format.find(".").is_some() {
    //            if value.parse::<f64>().unwrap() as usize as f64 == value.parse::<f64>().unwrap() && format.find(".").is_some() {
    //                let format_collect:Vec<&str> = format.split('.').collect();
    //                let pow = 10i32.pow(format_collect.get(1).unwrap().len() as u32);
    //                value = format!("{}", value.parse::<i32>().unwrap() * pow);
    //            }
    //            value = complex_number_format_mask(&value.parse::<f64>().unwrap(), &format, &true);
    //        } else {
    //            value = format!("{:0width$.len$}", value, width = min_width, len = right.len());
    //            value = Regex::new(&number_regex).unwrap().replace_all(&format, value.as_str());
    //        }
    //    }
    //    value
}

fn _merge_complex_number_format_masks(numbers: &Vec<String>, masks: &Vec<String>) -> Vec<String> {
    let mut decimal_count = numbers[1].len();
    let mut post_decimal_masks: Vec<String> = Vec::new();

    for mask in masks.iter().rev() {
        post_decimal_masks.push(mask.to_string());
        decimal_count -= mask.to_string().len();
        if decimal_count <= 0 {
            break;
        }
    }

    post_decimal_masks.reverse();
    let mut result: Vec<String> = Vec::new();
    result.push(masks.join("."));
    result.push(post_decimal_masks.join("."));
    result
}

fn _process_complex_number_format_mask(number: &f64, mask: &str) -> String {
    let mut result = number.to_string();
    let mut mask = mask.to_string();
    let re = Regex::new(r#"0+"#).unwrap();
    let mut masking_blocks: Vec<(String, usize)> = Vec::new();
    let mut masking_str: Vec<String> = Vec::new();
    let mut masking_beg: Vec<usize> = Vec::new();
    for ite in re.captures(&mask).unwrap().iter() {
        masking_str.push(ite.unwrap().to_string());
    }
    for (_, pos) in re.captures(&mask).unwrap().iter_pos().enumerate() {
        let (beg, _) = pos.unwrap();
        masking_beg.push(beg);
    }
    for i in 0..masking_str.len() {
        masking_blocks.push((
            masking_str.get(i).unwrap().clone(),
            *masking_beg.get(i).unwrap(),
        ));
    }

    if masking_blocks.len() > 1 {
        let mut number = *number;
        let mut offset: usize = 0;
        for (block, pos) in masking_blocks.iter().rev() {
            let divisor = format!("{}{}", 1, block).parse::<f64>().unwrap();
            let size = block.len();
            offset = *pos;

            let block_value = format!("{:0width$}", (&number % &divisor), width = size);

            number = (number / divisor) as f64;
            let from: String = mask.chars().skip(offset).take(size).collect();
            mask = mask.replace(&from, &block_value);
        }
        if number > 0f64 {
            let from: String = mask.chars().skip(offset).collect();
            mask = mask.replace(&from, &number.to_string());
        }
        result = mask;
    }
    result
}

fn _complex_number_format_mask(number: &f64, mask: &str, split_on_point: &bool) -> String {
    let sign = number < &0.0;
    let number = number.abs();

    if split_on_point == &true && mask.find('.').is_some() && number.to_string().find('.').is_some()
    {
        let number_str = number.to_string();
        let numbers_as: Vec<&str> = number_str.split('.').collect();
        let mut numbers: Vec<String> = Vec::new();
        for n in numbers_as {
            numbers.push(n.to_string());
        }
        let masks_as: Vec<&str> = mask.split('.').collect();
        let mut masks: Vec<String> = Vec::new();
        for mask in masks_as {
            masks.push(mask.to_string());
        }
        if masks.len() > 2 {
            masks = _merge_complex_number_format_masks(&numbers, &masks);
        }
        let result1 =
            _complex_number_format_mask(&numbers[0].parse::<f64>().unwrap(), &masks[0], &false);
        let result2 = _complex_number_format_mask(
            &numbers[1]
                .chars()
                .rev()
                .collect::<String>()
                .parse::<f64>()
                .unwrap(),
            &masks[1].chars().rev().collect::<String>(),
            &false,
        )
        .chars()
        .rev()
        .collect::<String>();

        return format!("{}{}.{}", if sign { "-" } else { "" }, result1, result2);
    }

    let result = _process_complex_number_format_mask(&number, mask);
    format!("{}{}", if sign { "-" } else { "" }, result)
}

fn gcd(a: &f64, b: &f64) -> f64 {
    if b == &0f64 {
        *a
    } else {
        gcd(b, &(a % b))
    }
}

#[test]
fn test_to_formatted_string_date() {
    let value = String::from("45435"); // 2024/5/23
    assert_eq!(
        r#"2024-05-23"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_YYYYMMDD2)
    );
    assert_eq!(
        r#"2024-05-23"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_YYYYMMDD)
    );
    assert_eq!(
        r#"23-05-2024"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DDMMYYYY)
    );
    assert_eq!(
        r#"23/05/2024"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DDMMYYYYSLASH)
    );
    assert_eq!(
        r#"23/5/24"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DMYSLASH)
    );
    assert_eq!(
        r#"23-5-24"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DMYMINUS)
    );
    assert_eq!(
        r#"23-5"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DMMINUS)
    );
    assert_eq!(
        r#"5-24"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_MYMINUS)
    );
    assert_eq!(
        r#"05-23-24"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX14)
    );
    assert_eq!(
        r#"23-May-24"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX15)
    );
    assert_eq!(
        r#"23-May"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX16)
    );
    assert_eq!(
        r#"May-24"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX17)
    );
    assert_eq!(
        r#"5/23/24 0:00"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX22)
    );
    assert_eq!(
        r#"23/5/24 0:00"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DATETIME)
    );
    assert_eq!(
        r#"12:00 am"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME1)
    );
    assert_eq!(
        r#"12:00:00 am"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME2)
    );
    assert_eq!(
        r#"0:00"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME3)
    );
    assert_eq!(
        r#"0:00:00"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME4)
    );
    assert_eq!(
        r#"00:00"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME5)
    );
    assert_eq!(
        r#"0:00:00"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME6)
    );
    assert_eq!(
        r#"0:00:00"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME8)
    );
    assert_eq!(
        r#"2024/05/23"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_YYYYMMDDSLASH)
    );

    let value = String::from("44349.211134259262"); // 2021/06/02 05:04:02
    assert_eq!(
        r#"2021-06-02"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_YYYYMMDD2)
    );
    assert_eq!(
        r#"2021-06-02"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_YYYYMMDD)
    );
    assert_eq!(
        r#"02-06-2021"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DDMMYYYY)
    );
    assert_eq!(
        r#"02/06/2021"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DDMMYYYYSLASH)
    );
    assert_eq!(
        r#"2/6/21"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DMYSLASH)
    );
    assert_eq!(
        r#"2-6-21"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DMYMINUS)
    );
    assert_eq!(
        r#"2-6"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DMMINUS)
    );
    assert_eq!(
        r#"6-21"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_MYMINUS)
    );
    assert_eq!(
        r#"06-02-21"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX14)
    );
    assert_eq!(
        r#"2-Jun-21"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX15)
    );
    assert_eq!(
        r#"2-Jun"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX16)
    );
    assert_eq!(
        r#"Jun-21"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX17)
    );
    assert_eq!(
        r#"6/2/21 5:04"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_XLSX22)
    );
    assert_eq!(
        r#"2/6/21 5:04"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_DATETIME)
    );
    assert_eq!(
        r#"5:04 am"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME1)
    );
    assert_eq!(
        r#"5:04:02 am"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME2)
    );
    assert_eq!(
        r#"5:04"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME3)
    );
    assert_eq!(
        r#"5:04:02"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME4)
    );
    assert_eq!(
        r#"04:02"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME5)
    );
    assert_eq!(
        r#"5:04:02"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME6)
    );
    assert_eq!(
        r#"5:04:02"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_TIME8)
    );
    assert_eq!(
        r#"2021/06/02"#,
        to_formatted_string(value.as_str(), NumberingFormat::FORMAT_DATE_YYYYMMDDSLASH)
    );
}
