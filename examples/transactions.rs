use fred::prelude::*;

#[tokio::main]
async fn main() -> Result<(), RedisError> {
  let config = RedisConfig::default();
  let client = RedisClient::new(config, None, None);

  let _ = client.connect();
  let _ = client.wait_for_connect().await?;
  let _ = client.flushall(false).await?;

  let trx = client.multi();
  let res1: RedisValue = trx.get("foo").await?;
  assert!(res1.is_queued());
  let res2: RedisValue = trx.set("foo", "bar", None, None, false).await?;
  assert!(res2.is_queued());
  let res3: RedisValue = trx.get("foo").await?;
  assert!(res3.is_queued());

  let values: (Option<String>, (), String) = trx.exec(true).await?;
  println!("Transaction results: {:?}", values);

  let _ = client.quit().await?;
  Ok(())
}
