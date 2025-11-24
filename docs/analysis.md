# Analysis process

Analyzing (unknown) firmware means that we need to find or build tools to help
us look at the given binary. Common utilities are hex viewers and editors, such
`xxd`, `hexdump` and `hexedit`, or the [ImHex app](https://imhex.werwolv.net/).
In addition, to help with [unpacking](unpacking.md), `dd`, `unxz`, `unlzma` and
similar utilities to cut out and decompress data are very handy.

## Recognizing data

The following example shall help understanding the thought process when trying
to get behind the meaning of unknown data. Mind that this takes a lot of time.
It often starts with the simple question: What is this?

Often enough, other researchers have already performed initial work to build on
top of. In the case of Intel ME generation 3 hardware, there are manifests with
lots of metadata, described through what are called _extensions_ by Positive
Technologies. They have created the following utilities:

- https://github.com/ptresearch/unME11
- https://github.com/ptresearch/unME12

### New extension in CPD manifest: `0x30`

With ME version 15 firmware, there are new manifest extensions. The following is
a hex dump of the data described by extension `0x30`, as printed by `xxd`.
Additional spaces and markers are there to assist the elaboration:

```
00196500:          [fd01 0000]>[3082]01f9 a003 0201  ........0.......
00196510: 0202 0101 300a 0608   2a86 48ce 3d04 0303  ....0...*.H.=...
00196520: 301a 3118 3016 0603   5504 030c 0f43 534d  0.1.0...U....CSM
00196530: 4520 4d43 4320 524f   4d20 4341 301e 170d  E MCC ROM CA0...
00196540: 3230 3131 3235 3030   3030 3030 5a17 0d34  201125000000Z..4
00196550: 3931 3233 3132 3335   3935 395a 3023 3121  91231235959Z0#1!
00196560: 301f 0603 5504 030c   1843 534d 4520 4d43  0...U....CSME MC
00196570: 4320 5356 4e30 3120   4b65 726e 656c 2043  C SVN01 Kernel C
00196580: 4130 7630 1006 072a   8648 ce3d 0201 0605  A0v0...*.H.=....
00196590: 2b81 0400 2203 6200   04aa aaaa aaaa aaaa  +...".b.........
001965a0: aaaa aaaa aaaa aaaa   aaaa aaaa aaaa aaaa  ................
001965b0: aaaa aaaa aaaa aaaa   aaaa aaaa aaaa aaaa  ................
001965c0: aaaa aaaa aaaa aaaa   aabb bbbb bbbb bbbb  ................
001965d0: bbbb bbbb bbbb bbbb   bbbb bbbb bbbb bbbb  ................
001965e0: bbbb bbbb bbbb bbbb   bbbb bbbb bbbb bbbb  ................
001965f0: bbbb bbbb bbbb bbbb   bba3 8201 0830 8201  .............0..
00196600: 0430 1f06 0355 1d23   0418 3016 8014 dddd  .0...U.#..0.....
00196610: dddd dddd dddd dddd   dddd dddd dddd dddd  ................
00196620: dddd 301d 0603 551d   0e04 1604 14cc cccc  ..0...U.........
00196630: cccc cccc cccc cccc   cccc cccc cccc cccc  ................
00196640: cc30 0f06 0355 1d13   0101 ff04 0530 0301  .0...U.......0..
00196650: 01ff 300e 0603 551d   0f01 01ff 0404 0302  ..0...U.........
00196660: 02ac 3081 a006 0355   1d1f 0481 9830 8195  ..0....U.....0..
00196670: 3081 92a0 4aa0 4886   4668 7474 7073 3a2f  0...J.H.Fhttps:/
00196680: 2f74 7363 692e 696e   7465 6c2e 636f 6d2f  /tsci.intel.com/
00196690: 636f 6e74 656e 742f   4f6e 4469 6543 412f  content/OnDieCA/
001966a0: 6372 6c73 2f4f 6e44   6965 5f43 415f 4353  crls/OnDie_CA_CS
001966b0: 4d45 5f49 6e64 6972   6563 742e 6372 6ca2  ME_Indirect.crl.
001966c0: 44a4 4230 4031 2630   2406 0355 040b 0c1d  D.B0@1&0$..U....
001966d0: 4f6e 4469 6520 4341   2043 534d 4520 496e  OnDie CA CSME In
001966e0: 7465 726d 6564 6961   7465 2043 4131 1630  termediate CA1.0
001966f0: 1406 0355 0403 0c0d   7777 772e 696e 7465  ...U....www.inte
00196700: 6c2e 636f 6dff ffff   0a00 0000 4800 0000  l.com.......H...
                     ^
```

The first 4 bytes look like the data size (in little endian). We can quickly
see that the actual data is a bit more than `0x200` bytes, so `0x01fd` is just
below that. A few bytes remain. But what comes next? The ASCII strings suggest
that it is something about certificates, often using encodings like ASN.1.
The encoding is specified in [RFC5280](https://www.ietf.org/rfc/rfc5280.txt).
For example, here we have:

- `30`: sequence
- `82`: length in octets

Let us skip the first 4 bytes, and try to have `openssl` parse it, assuming DER:

```sh
dd if=unk30.bin bs=1 skip=4 | openssl asn1parse -inform der
```

Which yields:
```
    0:d=0  hl=4 l= 505 cons: SEQUENCE
    4:d=1  hl=2 l=   3 cons: cont [ 0 ]
    6:d=2  hl=2 l=   1 prim: INTEGER           :02
    9:d=1  hl=2 l=   1 prim: INTEGER           :01
   12:d=1  hl=2 l=  10 cons: SEQUENCE
   14:d=2  hl=2 l=   8 prim: OBJECT            :ecdsa-with-SHA384
   24:d=1  hl=2 l=  26 cons: SEQUENCE
   26:d=2  hl=2 l=  24 cons: SET
   28:d=3  hl=2 l=  22 cons: SEQUENCE
   30:d=4  hl=2 l=   3 prim: OBJECT            :commonName
   35:d=4  hl=2 l=  15 prim: UTF8STRING        :CSME MCC ROM CA
   52:d=1  hl=2 l=  30 cons: SEQUENCE
   54:d=2  hl=2 l=  13 prim: UTCTIME           :201125000000Z
   69:d=2  hl=2 l=  13 prim: UTCTIME           :491231235959Z
   84:d=1  hl=2 l=  35 cons: SEQUENCE
   86:d=2  hl=2 l=  33 cons: SET
   88:d=3  hl=2 l=  31 cons: SEQUENCE
   90:d=4  hl=2 l=   3 prim: OBJECT            :commonName
   95:d=4  hl=2 l=  24 prim: UTF8STRING        :CSME MCC SVN01 Kernel CA
  121:d=1  hl=2 l= 118 cons: SEQUENCE
  123:d=2  hl=2 l=  16 cons: SEQUENCE
  125:d=3  hl=2 l=   7 prim: OBJECT            :id-ecPublicKey
  134:d=3  hl=2 l=   5 prim: OBJECT            :secp384r1
  141:d=2  hl=2 l=  98 prim: BIT STRING
  241:d=1  hl=4 l= 264 cons: cont [ 3 ]
  245:d=2  hl=4 l= 260 cons: SEQUENCE
  249:d=3  hl=2 l=  31 cons: SEQUENCE
  251:d=4  hl=2 l=   3 prim: OBJECT            :X509v3 Authority Key Identifier
  256:d=4  hl=2 l=  24 prim: OCTET STRING      [HEX DUMP]:30168014DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD
  282:d=3  hl=2 l=  29 cons: SEQUENCE
  284:d=4  hl=2 l=   3 prim: OBJECT            :X509v3 Subject Key Identifier
  289:d=4  hl=2 l=  22 prim: OCTET STRING      [HEX DUMP]:0414CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
  313:d=3  hl=2 l=  15 cons: SEQUENCE
  315:d=4  hl=2 l=   3 prim: OBJECT            :X509v3 Basic Constraints
  320:d=4  hl=2 l=   1 prim: BOOLEAN           :255
  323:d=4  hl=2 l=   5 prim: OCTET STRING      [HEX DUMP]:30030101FF
  330:d=3  hl=2 l=  14 cons: SEQUENCE
  332:d=4  hl=2 l=   3 prim: OBJECT            :X509v3 Key Usage
  337:d=4  hl=2 l=   1 prim: BOOLEAN           :255
  340:d=4  hl=2 l=   4 prim: OCTET STRING      [HEX DUMP]:030202AC
  346:d=3  hl=3 l= 160 cons: SEQUENCE
  349:d=4  hl=2 l=   3 prim: OBJECT            :X509v3 CRL Distribution Points
  354:d=4  hl=3 l= 152 prim: OCTET STRING      [HEX DUMP]:308195308192A04AA048864668747470733A2F2F747363692E696E74656C2E636F6D2F636F6E74656E742F4F6E44696543412F63726C732F4F6E4469655F43415F43534D455F496E6469726563742E63726CA244A442304031263024060355040B0C1D4F6E4469652043412043534D4520496E7465726D6564696174652043413116301406035504030C0D7777772E696E74656C2E636F6D
  509:d=0  hl=5 l=   0 cons: priv [ 2097034 ]
  514:d=0  hl=2 l=   0 prim: EOC
  ```

Success! Next up, we need to find a suitable library to parse this data. Further 
development is omitted here.

