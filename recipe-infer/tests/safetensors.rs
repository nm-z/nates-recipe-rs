use recipe_infer::safetensors::{
	Json, Member, decode, field_arr, field_str, parse_json, parse_safetensors,
};

#[test]
fn parse_safetensors_decodes_header_and_blob() {
	let header = concat!(
		r#"{"__metadata__":{"format":"pt"},"#,
		r#""a":{"dtype":"F32","shape":[2],"data_offsets":[0,8]},"#,
		r#""b":{"dtype":"F64","shape":[2],"data_offsets":[8,24]},"#,
		r#""c":{"dtype":"I64","shape":[2],"data_offsets":[24,40]}}"#,
	);
	let mut bytes = Vec::new();
	bytes.extend_from_slice(&(header.len() as u64).to_le_bytes());
	bytes.extend_from_slice(header.as_bytes());
	bytes.extend_from_slice(&1.5f32.to_le_bytes());
	bytes.extend_from_slice(&(-2.0f32).to_le_bytes());
	bytes.extend_from_slice(&3.0f64.to_le_bytes());
	bytes.extend_from_slice(&4.0f64.to_le_bytes());
	bytes.extend_from_slice(&(-7i64).to_le_bytes());
	bytes.extend_from_slice(&9i64.to_le_bytes());

	let parsed = parse_safetensors(&bytes).expect("parse safetensors image");
	assert_eq!(parsed.len(), 3);
	assert_eq!(parsed[0].0, "a");
	assert_eq!(parsed[0].1, vec![1.5, -2.0]);
	assert_eq!(parsed[1].0, "b");
	assert_eq!(parsed[1].1, vec![3.0, 4.0]);
	assert_eq!(parsed[2].0, "c");
	assert_eq!(parsed[2].1, vec![-7.0, 9.0]);
}

#[test]
fn parse_safetensors_rejects_out_of_range_offsets() {
	let header = r#"{"a":{"dtype":"F32","shape":[2],"data_offsets":[0,8]}}"#;
	let mut bytes = Vec::new();
	bytes.extend_from_slice(&(header.len() as u64).to_le_bytes());
	bytes.extend_from_slice(header.as_bytes());
	bytes.extend_from_slice(&1.5f32.to_le_bytes());
	assert!(parse_safetensors(&bytes).is_err());
}

#[test]
fn read_safetensors_header_and_decode_min() {
	use std::io::{Read, Seek, SeekFrom};
	let header = concat!(
		r#"{"a":{"dtype":"F32","shape":[2,2],"data_offsets":[0,16]},"#,
		r#""b":{"dtype":"F64","shape":[3],"data_offsets":[16,40]}}"#,
	);
	let mut bytes = Vec::new();
	bytes.extend_from_slice(&(header.len() as u64).to_le_bytes());
	bytes.extend_from_slice(header.as_bytes());
	for v in [1.5f32, -2.5, 3.0, 4.0] {
		bytes.extend_from_slice(&v.to_le_bytes());
	}
	for v in [10.0f64, 20.0, 30.0] {
		bytes.extend_from_slice(&v.to_le_bytes());
	}
	let path =
		std::env::temp_dir().join(format!("recipe_st_hdr_{}.safetensors", std::process::id()));
	std::fs::write(&path, &bytes).expect("write temp safetensors");

	let mut f = std::fs::File::open(&path).expect("open temp safetensors");
	let mut lb = [0u8; 8];
	f.read_exact(&mut lb).expect("read header len");
	let n = u64::from_le_bytes(lb);
	let mut hdr = vec![0u8; n as usize];
	f.read_exact(&mut hdr).expect("read header");
	let header_read = std::str::from_utf8(&hdr).expect("header utf8");
	let Json::Obj(entries) = parse_json(header_read).expect("parse header json") else {
		panic!("header not a JSON object");
	};
	let (mut count, mut params, mut min_bytes) = (0usize, 0u128, u64::MAX);
	let mut min_tensor: Option<(String, String, u64, u64)> = None;
	for Member { key: name, val } in &entries {
		if name == "__metadata__" {
			continue;
		}
		let Json::Obj(fields) = val else { continue };
		let dtype = field_str(fields, "dtype").unwrap_or_default();
		let shape = field_arr(fields, "shape").unwrap_or_default();
		let offs = field_arr(fields, "data_offsets").unwrap_or_default();
		count += 1;
		params += shape.iter().map(|&d| d as u128).product::<u128>();
		if offs.len() == 2 {
			let (b, e) = (offs[0] as u64, offs[1] as u64);
			if e > b && e - b < min_bytes {
				min_bytes = e - b;
				min_tensor = Some((name.clone(), dtype, b, e));
			}
		}
	}
	assert_eq!(count, 2, "two tensors parsed from the header");
	assert_eq!(params, 7, "2*2 + 3 element counts");

	let data_start = 8 + n;
	let (name, dtype, begin, end) = min_tensor.expect("a smallest tensor by byte span");
	assert_eq!(name, "a", "F32 [2,2] 16B is smaller than F64 [3] 24B");
	assert_eq!(dtype, "F32");
	f.seek(SeekFrom::Start(data_start + begin))
		.expect("seek tensor");
	let mut raw = vec![0u8; (end - begin) as usize];
	f.read_exact(&mut raw).expect("read tensor bytes");
	let vals = decode(&dtype, &raw).expect("decode F32 tensor");
	assert_eq!(
		vals,
		vec![1.5, -2.5, 3.0, 4.0],
		"decoded F32 tensor, widened to f64"
	);
	let _ = std::fs::remove_file(&path);
}
