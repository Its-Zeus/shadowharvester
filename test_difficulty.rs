fn difficulty_to_zero_bits(difficulty_hex: &str) -> usize {
    let difficulty_bytes = hex::decode(difficulty_hex).unwrap();
    let mut zero_bits = 0;
    for &byte in difficulty_bytes.iter() {
        if byte == 0x00 {
            zero_bits += 8;
        } else {
            zero_bits += byte.leading_zeros() as usize;
            break;
        }
    }
    zero_bits
}

fn hash_structure_good(hash: &[u8], zero_bits: usize) -> bool {
    let full_bytes = zero_bits / 8;
    let remaining_bits = zero_bits % 8;

    if hash.len() < full_bytes || hash[..full_bytes].iter().any(|&b| b != 0) {
        return false;
    }

    if remaining_bits == 0 {
        return true;
    }
    if hash.len() > full_bytes {
        let mask = 0xFF << (8 - remaining_bits);
        hash[full_bytes] & mask == 0
    } else {
        false
    }
}

fn main() {
    // Test with common difficulty values
    let difficulties = vec!["00FFFFFF", "000FFFFF", "0000FFFF", "00007FFF"];
    
    for diff in difficulties {
        let zero_bits = difficulty_to_zero_bits(diff);
        println!("Difficulty: {} → {} zero bits", diff, zero_bits);
    }
    
    // Test hash validation
    let test_hash1 = [0x00, 0x00, 0x00, 0xFF]; // 24 zero bits
    let test_hash2 = [0x00, 0x00, 0x7F, 0xFF]; // 17 zero bits
    let test_hash3 = [0x00, 0x00, 0x00, 0x7F]; // 25 zero bits
    
    println!("\nTest hash [0x00, 0x00, 0x00, 0xFF] with 24 zero bits: {}", hash_structure_good(&test_hash1, 24));
    println!("Test hash [0x00, 0x00, 0x7F, 0xFF] with 17 zero bits: {}", hash_structure_good(&test_hash2, 17));
    println!("Test hash [0x00, 0x00, 0x00, 0x7F] with 25 zero bits: {}", hash_structure_good(&test_hash3, 25));
}
