use super::RispExp;
use super::RispErr;

pub fn parse_list_of_floats(args: &[RispExp]) -> Result<Vec<f64>, RispErr> {
  args
    .iter()
    .map(|x| parse_single_float(x))
    .collect()
}

pub fn parse_single_float(exp: &RispExp) -> Result<f64, RispErr> {
  match exp {
    RispExp::Number(num) => Ok(*num),
    _ => Err(RispErr::Reason("expected a number".to_string())),
  }
}

pub fn plus_func() -> RispExp {
    RispExp::Func(
	|args: &[RispExp]| -> Result<RispExp, RispErr> {
            let sum = parse_list_of_floats(args)?.iter().fold(0.0, |sum, a| sum + a);
            Ok(RispExp::Number(sum))
	}
    )
}

pub fn mult_func() -> RispExp {
    RispExp::Func(
	|args: &[RispExp]| -> Result<RispExp, RispErr> {
            let floats = parse_list_of_floats(args)?;
	    let first = *floats.first().ok_or(RispErr::Reason("expected at least one number".to_string()))?;
            let product = floats[1..].iter().fold(first, |sum, a| sum * a);
            Ok(RispExp::Number(product))
	}
    )
}

pub fn minus_func() -> RispExp {
    RispExp::Func(
	|args: &[RispExp]| -> Result<RispExp, RispErr> {
            let floats = parse_list_of_floats(args)?;
            let first = *floats.first().ok_or(RispErr::Reason("expected at least one number".to_string()))?;
            let sum_of_rest = floats[1..].iter().fold(0.0, |sum, a| sum + a);
            Ok(RispExp::Number(first - sum_of_rest))
	}
    )
}

// End

