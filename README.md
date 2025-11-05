# Scavenger Mine API Documentation

**Version:** 1.0  
**Last Updated:** October 2025  
**Provider:** Midnight TGE Ltd.

---

## Table of Contents

1. [Overview](#overview)
2. [System Overview](#system-overview)
3. [API Endpoints](#api-endpoints)
   - [GET /TandC](#get-tandc)
   - [POST /register](#post-register)
   - [GET /challenge](#get-challenge)
   - [POST /solution](#post-solution)
   - [POST /donate_to](#post-donate_to)
   - [GET /work_to_star_rate](#get-work_to_star_rate)
4. [AshMaize Hash Algorithm](#ashmaize-hash-algorithm)
5. [Error Responses](#error-responses)

---

## Overview

### Purpose

This API enables participation in the Scavenger Mine program without using the browser interface. The Scavenger Mine is a 21-day proof-of-work system designed for participation with ordinary computers, without requiring specialized hardware.

### Audience

This documentation is intended for developers with:
- Software development skills
- Understanding of blockchain principles
- Knowledge of cryptographic concepts

For non-technical users, the browser interface is recommended.

### Base URL

```
https://scavenger.prod.gd.midnighttge.io
```

---

## System Overview

### Key Features

- **Duration:** 21 days with 504 total challenges (24 per day)
- **Challenge Frequency:** New challenge every 60 minutes
- **Submission Window:** Up to 24 hours after challenge availability (except final day)
- **Registration:** Required before solutions are accepted; can register 24 hours before start
- **Proof System:** Server provides signed receipts for valid solutions
- **Daily Merkle Trees:** Root submitted to Cardano mainnet
- **No Pre-mine Guarantee:** Uses unpredictable daily Merkle roots and pre-registrations

### Mining Phases

- **Day 0:** Pre-registration period (24 hours before start)
- **Days 1-21:** Active mining period
- **Day 22:** 24-hour consolidation period

---

## API Endpoints

### GET /TandC

Retrieve the Token End User Agreement (terms and conditions).

#### Endpoint

```
GET /TandC/{version}
```

#### Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| version | string | No | "1-0" | Version of terms to retrieve |

#### Authentication

None required (public endpoint)

#### Availability

Days 0, 1-21, 22

#### Response

```json
{
  "version": "1-0",
  "content": "**TOKEN END-USER TERMS**\n\nLast updated: 15 Aug 2025...",
  "message": "I agree to abide by the terms and conditions as described in version 1-0 of the Midnight scavenger mining process: 281ba5f69f4b943e3fb8a20390878a232787a04e4be22177f2472b63df01c200"
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| version | string | Version identifier (e.g., "1-0") |
| content | string | Full text of terms and conditions |
| message | string | Pre-formatted acceptance message with hash |

#### Example Request

```bash
curl https://scavenger.prod.gd.midnighttge.io/TandC
```

#### Error Response (404)

```json
{
  "message": "Terms and Conditions version 2-0 not found",
  "error": "Not Found",
  "statusCode": 404
}
```

---

### POST /register

Register a Cardano destination address to participate in Scavenger Mine.

#### Endpoint

```
POST /register/{address}/{signature}/{pubkey}
```

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| address | string | Yes | Standard Cardano payment address |
| signature | string | Yes | CIP 8/30 signature over the T&C message |
| pubkey | string | Yes | 64-character hex encoded public key |

#### Authentication

Signature verification via CIP 8/30

#### Availability

Days 0, 1-21, 22

#### Signature Message

The signature must be created over the exact message returned from `GET /TandC`:

```
I agree to abide by the terms and conditions as described in version 1-0 of the Midnight scavenger mining process: 281ba5f69f4b943e3fb8a20390878a232787a04e4be22177f2472b63df01c200
```

#### Response

```json
{
  "registrationReceipt": {
    "preimage": "addr1qq4dl3nhr0axurgcrpun9xyp04pd2r2dwu5x7eeam98psv6dhxlde8ucclv2p46hm077ds4vzelf5565fg3ky794uhrq5up0he...",
    "signature": "8d622d59ef73f49c16d627994dfde8f7632fde5d064c32acfde4a5427f5d10dd...",
    "timestamp": "2025-10-29T20:45:35.425Z"
  }
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| registrationReceipt | object | Receipt confirming registration |
| registrationReceipt.preimage | string | Concatenation of address, signature, and timestamp |
| registrationReceipt.signature | string | Server's signature over the preimage |
| registrationReceipt.timestamp | string | ISO 8601 timestamp of registration |

#### Example Request

```bash
curl -X POST \
  "https://scavenger.prod.gd.midnighttge.io/register/{address}/{signature}/{pubkey}" \
  -d "{}"
```

#### Error Responses

**Invalid Public Key Format (400)**
```json
{
  "message": "Invalid pubkey format - must be 64-character hex string",
  "error": "Bad Request",
  "statusCode": 400
}
```

**Wrong Network (400)**
```json
{
  "message": "CIP-30 signature verification failed: Wrong network: expected preprod, got mainnet",
  "error": "Bad Request",
  "statusCode": 400
}
```

**Invalid Signature Message (400)**
```json
{
  "message": "CIP-30 signature verification failed: Message in signature does not match provided message",
  "error": "Bad Request",
  "statusCode": 400
}
```

#### Notes

- Use short-form (64-character) public key, not the long-form version
- Signature can be validated at: https://verifycardanomessage.cardanofoundation.org/
- Each address represents signed acceptance of Token End User Agreement
- Registration is required once per destination address

---

### GET /challenge

Fetch the currently available challenge.

#### Endpoint

```
GET /challenge
```

#### Parameters

None

#### Authentication

None required (public endpoint)

#### Availability

Days 1-21

#### Response (Active Mining)

```json
{
  "code": "active",
  "challenge": {
    "challenge_id": "**D21C10",
    "day": 21,
    "challenge_number": 10,
    "issued_at": "2025-11-20T11:00:00.000Z",
    "latest_submission": "2025-11-20T23:59:59Z",
    "difficulty": "0000FFFF",
    "no_pre_mine": "cddba7b592e3133393c16194fac7431abf2f5485ed711db282183c819e08ebaa",
    "no_pre_mine_hour": "548571128"
  },
  "mining_period_ends": "2025-11-20T23:59:59Z",
  "max_day": 21,
  "total_challenges": 504,
  "current_day": 21,
  "next_challenge_starts_at": "2025-11-20T20:00:00Z"
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| code | string | Status: "before", "active", or "after" |
| challenge | object | Current challenge details |
| challenge.challenge_id | string | Unique challenge identifier (format: **DDCC) |
| challenge.day | integer | Day number (1-21) |
| challenge.challenge_number | integer | Challenge number (1-504) |
| challenge.issued_at | string | ISO 8601 timestamp when challenge was issued |
| challenge.latest_submission | string | ISO 8601 deadline for submissions |
| challenge.difficulty | string | 4-byte hex mask for solution validation |
| challenge.no_pre_mine | string | Hex string for AshMaize ROM initialization |
| challenge.no_pre_mine_hour | string | Hourly randomness value |
| mining_period_ends | string | ISO 8601 end time for all mining |
| max_day | integer | Total days (21) |
| total_challenges | integer | Total challenges (504) |
| current_day | integer | Current day (1-21) |
| next_challenge_starts_at | string | ISO 8601 time of next challenge |

#### Example Request

```bash
curl https://scavenger.prod.gd.midnighttge.io/challenge
```

#### Response Before Mining (Day 0)

```json
{
  "code": "before",
  "starts_at": "2025-10-30T00:00:00.000Z"
}
```

#### Response After Mining (Day 22)

```json
{
  "code": "after"
}
```

#### Expected Response Size

Approximately 554 bytes

---

### POST /solution

Submit a solution to a challenge.

#### Endpoint

```
POST /solution/{address}/{challenge_id}/{nonce}
```

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| address | string | Yes | Registered Cardano address |
| challenge_id | string | Yes | Challenge ID from GET /challenge |
| nonce | string | Yes | 16-character hex encoded 64-bit number |

#### Authentication

Address must be previously registered

#### Availability

Days 1-21

#### Solution Creation Process

The nonce is created through the following process:

1. **Initialize AshMaize ROM** with `no_pre_mine` from challenge
2. **Construct preimage** by concatenating:
   - `nonce` (64-bit hex)
   - `address` (your registered address)
   - `challenge_id` (from challenge)
   - `difficulty` (from challenge)
   - `no_pre_mine` (from challenge)
   - `latest_submission` (from challenge)
   - `no_pre_mine_hour` (from challenge)
3. **Hash preimage** with AshMaize
4. **Compare** zero bits in leftmost 4 bytes of hash with difficulty
5. **Iterate** by changing nonce until match found
6. **Submit** solution with address, challenge_id, and nonce

#### Response (Success)

```json
{
  "crypto_receipt": {
    "preimage": "0019c96b6a30ee38addr_test1qq4dl3nhr0axurgcrpun9xyp04pd2r2dwu5x7eeam98psv6dhxlde8ucclv2p46hm077ds4vzelf5565fg3ky794uhrq5up0he**D07C10000FFFFFfd651ac2725e3b9d804cc8b161c0709af14d6264f93e8d4afef0fd1142a3f0112025-10-19T08:59:59.000Z509681483",
    "timestamp": "2025-10-18T09:01:51.249Z",
    "signature": "4904f6d82f7075ac21dec9e9c29f6e68d9f36608cc7e7f7912cc62eb4b1b7cf96a83998e8268afb5a6cefedec3331247868d2ae1cee4a736091b3b5acfd25202"
  }
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| crypto_receipt | object | Signed receipt of valid solution |
| crypto_receipt.preimage | string | Complete preimage data |
| crypto_receipt.timestamp | string | ISO 8601 timestamp of acceptance |
| crypto_receipt.signature | string | Server signature over preimage + timestamp |

#### Example Request

```bash
curl -X POST \
  "https://scavenger.prod.gd.midnighttge.io/solution/{address}/{challenge_id}/{nonce}" \
  -d "{}"
```

#### Error Responses

**Address Not Registered (400)**
```json
{
  "statusCode": 400,
  "message": "Solution validation failed: Address is not registered"
}
```

**Challenge Not Found (404)**
```json
{
  "statusCode": 404,
  "message": "Challenge not found: <supplied id>"
}
```

**Solution Does Not Meet Difficulty (400)**
```json
{
  "statusCode": 400,
  "message": "Solution validation failed: Solution does not meet difficulty"
}
```

#### Notes

- The crypto_receipt can be used by auditors to verify solution validity and submission time
- The nonce is the only element that can be changed for a given challenge/address pair
- Solutions must meet the difficulty requirement (matching zero bits)

---

### POST /donate_to

Consolidate solutions from one address to another address.

#### Endpoint

```
POST /donate_to/{destination_address}/{original_address}/{signature}
```

#### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| destination_address | string | Yes | Registered address to receive consolidated NIGHT |
| original_address | string | Yes | Registered address donating its tokens |
| signature | string | Yes | CIP 8/30 signature from original_address |

#### Authentication

Signature created by original_address's private key

#### Availability

Days 1-21, 22

#### Signature Message Format

```
Assign accumulated Scavenger rights to: {destination_address}
```

#### Use Case

Consolidate multiple participating addresses into a single address for:
- Simplified NIGHT management from one wallet
- Reduced transaction fees
- Lower minimum ADA requirements
- Fewer redemption transactions during thaw periods

#### Response (Success - No Prior Solutions)

```json
{
  "status": "success",
  "message": "Successfully assigned accumulated Scavenger rights from addr1q... to addr1q...",
  "donation_id": "123e4567-e89b-12d3-a456-426614174000",
  "original_address": "addr1q...",
  "destination_address": "addr1q...",
  "timestamp": "2025-08-15T10:30:00.000Z",
  "solutions_consolidated": 0
}
```

#### Response (Success - With Solutions)

```json
{
  "status": "success",
  "message": "Successfully assigned accumulated Scavenger rights from addr_test1qrv3cp0m9u7y0... to addr_test1qq4dl3nhr...",
  "donation_id": "48a9e58e-139b-4a74-94dd-a37096bd35da",
  "original_address": "addr_test1qrv3cp0...",
  "destination_address": "addr_test1qq4dl3nhr...",
  "timestamp": "2025-10-16T14:47:33.925Z",
  "solutions_consolidated": 2
}
```

#### Response (Undo Assignment - Donate to Self)

```json
{
  "status": "success",
  "message": "Successfully undid donation assignment for addr_test1qrv3cp0m9u7y0...",
  "original_address": "addr_test1qrv3cp0...",
  "destination_address": "addr_test1qrv3cp0...",
  "timestamp": "2025-10-16T14:11:02.200Z",
  "solutions_consolidated": 0
}
```

#### Response Fields

| Field | Type | Description |
|-------|------|-------------|
| status | string | "success" or error status |
| message | string | Human-readable result description |
| donation_id | string | UUID identifying this consolidation |
| original_address | string | Source address |
| destination_address | string | Target address |
| timestamp | string | ISO 8601 timestamp of operation |
| solutions_consolidated | integer | Number of solutions moved |

#### Example Request

```bash
curl -L -X POST \
  "https://scavenger.prod.gd.midnighttge.io/donate_to/{destination_address}/{original_address}/{signature}" \
  -d "{}"
```

#### Error Responses

**Already Has Active Donation (409)**
```json
{
  "message": "Original address addr_test1qrv3cp0... already has an active donation assignment to addr_test1qq4dl3nhr...",
  "error": "Conflict",
  "statusCode": 409
}
```

**Invalid Signature (400)**
```json
{
  "message": "Invalid CIP-30 signature failed: Message in signature does not match provided message...",
  "error": "Bad Request",
  "statusCode": 400
}
```

**Original Address Not Registered (404)**
```json
{
  "message": "Original address addr1qrv3cp0m9u... is not registered",
  "error": "Not Found",
  "statusCode": 404
}
```

#### Important Notes

- Consolidation applies to ALL claims (past and future) from original_address
- Only simple one-level donations allowed (no chains)
- Can be performed anytime during mining and 24 hours after closure
- Donating to self undoes a previous donation assignment
- Server validates signature using public key from original_address registration

---

### GET /work_to_star_rate

Get the STAR allocation rate for each successful solution.

#### Endpoint

```
GET /work_to_star_rate
```

#### Parameters

None

#### Authentication

None required (public endpoint)

#### Availability

Days 0, 1-21, 22

#### Response (Example from Day 4)

```json
[10882519, 7692307, 12487254]
```

#### Response Format

Array of integers representing STAR per solution for each completed day.

- Array index corresponds to day number (0-indexed)
- Each value is the STAR allotment per crypto_receipt for that day
- Empty array `[]` returned on Days 0 and 1

#### STAR Calculation

1 NIGHT = 1,000,000 STAR (smallest unit)

To calculate total STAR for an address:
```
Total STAR = Σ(solutions_per_day × star_rate_for_day)
```

#### Example Calculation

Address 'A' successfully solves:
- Day 1: 24 challenges
- Day 2: 24 challenges  
- Day 3: 20 challenges

```
Day 1: 24 × 10,882,519 = 261,180,456 STAR
Day 2: 24 × 7,692,307 = 184,615,368 STAR
Day 3: 20 × 12,487,254 = 249,745,080 STAR
Total: 695,540,904 STAR = 695.540904 NIGHT
```

#### Example Request

```bash
curl https://scavenger.prod.gd.midnighttge.io/work_to_star_rate
```

#### Notes

- Rates vary by day based on total network participation
- Use this endpoint to estimate potential earnings
- Actual NIGHT allocation determined after each day's Merkle tree submission

---

## AshMaize Hash Algorithm

### Overview

AshMaize is an ASIC-resistant variant of the RandomX hash algorithm, enabling participation with ordinary computers without specialized hardware.

### Configuration

Use these exact configuration values for compatibility:

| Parameter | Value |
|-----------|-------|
| nbLoops | 8 |
| nbInstrs | 256 |
| pre_size | 16777216 |
| mixing_numbers | 4 |
| rom_size | 1073741824 |

### ROM Initialization

Before hashing, initialize the AshMaize ROM with the `no_pre_mine` value from the current challenge. This value changes daily to ensure fairness.

### Test Vector

Given challenge data:
```json
{
  "challenge_id": "**D07C10",
  "latest_submission": "2025-10-19T08:59:59.000Z",
  "difficulty": "000FFFFF",
  "no_pre_mine": "fd651ac2725e3b9d804cc8b161c0709af14d6264f93e8d4afef0fd1142a3f011",
  "no_pre_mine_hour": "509681483"
}
```

For address:
```
addr_test1qq4dl3nhr0axurgcrpun9xyp04pd2r2dwu5x7eeam98psv6dhxlde8ucclv2p46hm077ds4vzelf5565fg3ky794uhrq5up0he
```

Example preimage:
```
0019c96b6a30ee38addr_test1qq4dl3nhr0axurgcrpun9xyp04pd2r2dwu5x7eeam98psv6dhxlde8ucclv2p46hm077ds4vzelf5565fg3ky794uhrq5up0he**D07C10000FFFFFfd651ac2725e3b9d804cc8b161c0709af14d6264f93e8d4afef0fd1142a3f0112025-10-19T08:59:59.000Z509681483
```

Expected hash:
```
000694200fb04137812fb7f35fab2f0e07adf8465397d268bcd97d2f4c7b875fe6d42f12f377b5b83bcfbd70d6ba55441650c37b8fc80851216b3a1aed7e23c8
```

Verification:
```
Difficulty:  000FFFFF
Hash prefix: 00069420
Match: ✓ (zero bits align)
```

### Source Code

Available at: https://github.com/input-output-hk/Ashmaize

---

## Error Responses

### Standard Error Format

All errors follow this JSON structure:

```json
{
  "message": "Descriptive error message",
  "error": "Error Type",
  "statusCode": 400
}
```

### Common HTTP Status Codes

| Code | Meaning | Common Causes |
|------|---------|---------------|
| 400 | Bad Request | Invalid parameters, signature verification failed |
| 404 | Not Found | Address not registered, challenge not found |
| 409 | Conflict | Duplicate registration, existing donation assignment |

### Unavailable Period Responses

**Before Mining (Day 0)**
```json
{
  "code": "before",
  "starts_at": "2025-10-30T00:00:00.000Z"
}
```

**After Mining (Day 22+)**
```json
{
  "code": "after"
}
```

---

## Best Practices

### Registration
- Register addresses at least 24 hours before mining starts
- Keep your private key secure - signatures prove ownership
- Use the correct network (mainnet vs testnet)

### Mining
- Poll `/challenge` endpoint every 60 minutes for new challenges
- Submit solutions within 24 hours of challenge issuance
- Store crypto_receipts as proof of successful solutions

### Consolidation
- Consolidate addresses during or up to 24 hours after mining
- Consider consolidating to minimize redemption transactions
- Cannot create chains - only one level of donation allowed

### Performance
- Initialize AshMaize ROM once per challenge (when `no_pre_mine` changes)
- Implement efficient nonce iteration strategies
- Cache challenge data to reduce API calls

---

## Rate Limits

Not explicitly documented. Implement exponential backoff for failed requests.

---

## Support

For questions about:
- **Anthropic API/Products:** https://docs.claude.com
- **General Support:** https://support.claude.com
- **Midnight TGE Ltd:** Refer to official Midnight documentation

---

## Legal Disclaimer

The information provided is for informational purposes only and should not be construed as financial, legal, or investment advice. Midnight TGE Ltd. and affiliates make no representations or warranties regarding accuracy, completeness, or reliability. Users are responsible for observing all applicable laws and regulations in their jurisdiction.

© 2025 Midnight TGE Ltd. All Rights Reserved

---

## Version History

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | October 2025 | Initial release |
