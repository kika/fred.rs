use fred::types::{Library, Script};
use fred::{prelude::*, util as fred_utils};

static SCRIPTS: &'static [&'static str] = &[
  "return {KEYS[1],KEYS[2],ARGV[1],ARGV[2]}",
  "return {KEYS[2],KEYS[1],ARGV[1],ARGV[2]}",
  "return {KEYS[1],KEYS[2],ARGV[2],ARGV[1]}",
  "return {KEYS[2],KEYS[1],ARGV[2],ARGV[1]}",
];

#[tokio::main]
async fn main() -> Result<(), RedisError> {
  let config = RedisConfig::default();
  let client = RedisClient::new(config, None, None);
  let _ = client.connect();
  let _ = client.wait_for_connect().await?;

  for script in SCRIPTS.iter() {
    let hash = fred_utils::sha1_hash(script);
    let mut script_exists: Vec<bool> = client.script_exists(&hash).await?;

    if !script_exists.pop().unwrap_or(false) {
      let _ = client.script_load(*script).await?;
    }

    let results = client.evalsha(&hash, vec!["foo", "bar"], vec![1, 2]).await?;
    println!("Script results for {}: {:?}", hash, results);
  }

  // or use eval without script_load
  let result = client.eval(SCRIPTS[0], vec!["foo", "bar"], vec![1, 2]).await?;
  println!("First script result: {:?}", result);

  let _ = client.quit().await;
  Ok(())
}

// or use the `Script` utility types
async fn scripts() -> Result<(), RedisError> {
  let config = RedisConfig::default();
  let client = RedisClient::new(config, None, None);
  let _ = client.connect();
  let _ = client.wait_for_connect().await?;

  let script = Script::from_lua(SCRIPTS[0]);
  let _ = script.load(&client).await?;
  let result = script.evalsha(&client, vec!["foo", "bar"], vec![1, 2]).await?;
  println!("First script result: {:?}", result);

  Ok(())
}

// or use the `Function` and `Library` utility types
async fn functions() -> Result<(), RedisError> {
  let config = RedisConfig::default();
  let client = RedisClient::new(config, None, None);
  let _ = client.connect();
  let _ = client.wait_for_connect().await?;

  let echo_lua = include_str!("../tests/scripts/lua/echo.lua");
  let lib = Library::from_code(&client, echo_lua).await?;
  let func = lib.functions().get("echo").expect("Failed to read echo function");

  let result: Vec<String> = func.fcall(&client, vec!["foo{1}", "bar{1}"], vec!["3", "4"]).await?;
  assert_eq!(result, vec!["foo{1}", "bar{1}", "3", "4"]);

  Ok(())
}
