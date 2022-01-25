use reqwest::blocking::get as httpget;
use super::RispExp;
use super::RispErr;
use super::RispValueString;

pub fn httpget_func() -> RispExp {
    RispExp::Func(|args: &[RispExp]| -> Result<RispExp, RispErr> {
	if args.len() < 1 {
	    return Err(RispErr::Reason("pass a url".to_string()));
	}
	let url = args[0].lisp_val();
	let url = if url == "test" {
	    "https://jsonplaceholder.typicode.com/posts/1".to_string()
	} else {
	    url
	};
	let res = match httpget(url) {
	    Ok(response) => Box::new(response),
	    Err(e) => return Err(RispErr::Reason(e.to_string())),
	};
	
	let status = res.status().as_u16() as f64;
	let res_url = res.url();
	let headers = res.headers();
	let mut header_list: Vec<RispExp> = Vec::new();
	for (name, value) in headers.iter() {
	  let mut pair = Vec::new();
	  pair.push(RispExp::Str(name.to_string()));
	  pair.push(RispExp::Str(value.to_str().unwrap().to_string()));
	  header_list.push(RispExp::List(pair));
	}

	let mut response_list: Vec<RispExp> = vec![
	  RispExp::Number(status),
	  RispExp::Str(res_url.to_string()),
	  RispExp::List(header_list)
	];

	let content_type = headers.get("content-type").unwrap().to_str().unwrap();
	if content_type.starts_with("application/json") {
	  let text_content = res.text_with_charset("utf-8").unwrap();
	  let json = match serde_json::from_str(&text_content) {
	    Ok(data) => data,
	    Err(e) => return Err(RispErr::Reason(e.to_string()))
	  };
	  let json = RispExp::Json(json);
	  response_list.push(json);
	}
	Ok(RispExp::List(response_list))
      }
    )
}

// End

