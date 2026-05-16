use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Generate an HMAC-SHA256 signature for the given query string using the secret key.
pub fn sign(secret_key: &str, message: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(secret_key.as_bytes()).expect("HMAC can accept any key length");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Append timestamp, recvWindow, and signature to a query string.
/// Returns the fully signed query string.
pub fn sign_query(secret_key: &str, query: &str, server_time: u64) -> String {
    let mut signed = if query.is_empty() {
        format!("recvWindow=50000&timestamp={}", server_time)
    } else {
        format!("{}&recvWindow=50000&timestamp={}", query, server_time)
    };

    let signature = sign(secret_key, &signed);
    signed.push_str(&format!("&signature={}", signature));
    signed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sign() {
        // Example from Bittime docs
        let secret = "NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j";
        let message = "symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&recvWindow=5000&timestamp=1499827319559";
        let sig = sign(secret, message);
        assert_eq!(
            sig,
            "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71"
        );
    }

    #[test]
    fn test_sign_query_empty() {
        let secret = "testsecret";
        let result = sign_query(secret, "", 1234567890);
        assert!(result.starts_with("recvWindow=50000&timestamp=1234567890"));
        assert!(result.contains("&signature="));
    }

    #[test]
    fn test_sign_query_with_params() {
        let secret = "testsecret";
        let result = sign_query(secret, "symbol=BTCUSDT", 1234567890);
        assert!(result.starts_with("symbol=BTCUSDT&recvWindow=50000&timestamp=1234567890"));
        assert!(result.contains("&signature="));
    }
}
