#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_m1_matches_js() {
        let email = "test@example.com";
        let salt = hex::decode("c71831de4be151915261ae1a24127846ce0117c58d05c3b792424cabce69c052")
            .unwrap();
        let a_pub = hex::decode("5029d310534ae41ca45b840f3e742879e999ce3aa34216063a1a30978d7ea4cbc8cd73287d065837a168b945754a7d9ef7f0b05abbe530b327e2d4e6006ead9fdfa71f91484272e53ef926422c19fb84dc1f8c2f484da029612f36f2ee8b296b9b86d46ca153d14c8ca46e515d365539f8d62a2fead86efb20e8cb0b12a68028968e90452ba3942f0d08f435741aa8a46a158663dc2e7719b614164c862511d9d15a51bafbd363f6dcf20083c16fddf40d3a6fffade10f566138ac63f8f8735d967ac7218a83c4fc1d5a696df8fe43a832cc95eed53d7d2e69583178a6d1df23830d1316d6281a8b5cb9f9cbc2a5e820e39525ffb4c6ebd227a53ce5f5abdc30").unwrap();
        let b_pub = hex::decode("132e13eba3b32d5eae53a78149ac22a7d20924e8800af68c95f1f1a104064f96047e8659ea6d25fd9217bd41331042ec080844b6af08d5c85c8cf67b1e2d5523368fab95b3cad74606a4938ad5d89ca5c179f92145ccebb27e3ed328e3d7fc2a8f7d996be59e77df8b06c27a0428d6854d657c0f0aa29c6352e56b31da669b03d43e53c187a84ca9ae52a2001121d7e5f925c731212bcbd97335242828d50e9007c4e91c87b6dfbe14a0006558230ef54379f3d6281f0676940e2359230de4e87f7a850459318990ada910dc1aa4821dde4dedc19b5fc408f233998b3d923463f90e9638f28e75c7e7e0258fc778a4446bff314c8e6cd1dba8351735c8ab81e6").unwrap();
        let session_key =
            hex::decode("ba22fca411d0b150fd7fe84b8981512c05251df092f97a468380eb1796c69f06")
                .unwrap();

        let expected_m1 = "d8e05194652688047c0acd1785fb793d2c8eca81dbd7de7aede00a4f78741ae6";

        let m1 = compute_m1(email, &salt, &a_pub, &b_pub, &session_key);
        let m1_hex = hex::encode(&m1);

        println!("Expected M1: {}", expected_m1);
        println!("Computed M1: {}", m1_hex);

        assert_eq!(m1_hex, expected_m1);
    }
}
