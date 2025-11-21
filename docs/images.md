# Firmware images

There are multiple ways of obtaining firmware images:

- read out from one's own mainboard, e.g. using
  [`flashprog`](https://flashprog.org/)
- download from a vendor; however, those are mostly _upgrade_ images, not the
  same as what would be found on real hardware, usually in a proprietary format
- download from an archive, obtain a full backup image, or similar

## Full images

### ChromeOS

The [coreboot project](https://coreboot.org) offers a utility to download full
recovery images for ChromeOS (Chromebooks) including a base system and firmware,
and extract the firmware image:
[`util/chromeos/crosfirmware.sh`](https://github.com/coreboot/coreboot/blob/main/util/chromeos/crosfirmware.sh)

To download all the images and extract the firmware, run:
`./crosfirmware.sh all`

As of the time of writing this, that will download:
<https://dl.google.com/dl/edgedl/chromeos/recovery/recovery.conf>

Which looks somewhat like this:

```
recovery_tool_version=0.9.2
recovery_tool_linux_version=0.9.2
recovery_tool_update=


name=Dell Chromebook 13 (3380)
version=15393.58.0
desc=
channel=STABLE
hwidmatch=^ASUKA .*
hwid=
md5=9e3788b775f0c55f37682a6db6add00a
sha1=b54a3762e22d08dc3b67846e3ab16bd7333e519d
zipfilesize=1275561266
file=chromeos_15393.58.0_asuka_recovery_stable-channel_mp-v2.bin
filesize=2330960384
url=https://dl.google.com/dl/edgedl/chromeos/recovery/chromeos_15393.58.0_asuka_recovery_stable-channel_mp-v2.bin.zip
```

and so on. Each entry is about 12 lines. How much is in there?

```
wc recovery.conf
  9186  11369 297858 recovery.conf
```

More than 650 images?! ... no, there are lots of duplicates. But not exactly.
Some recovery images contain firmware for multiple devices!

```
grep url= recovery.conf | uniq | wc
    76     76   9310
```

A good bunch of downloads would fail, though. I reduced the script to fewer
entries. Note that Chromebooks may be based on any platform, not only Intel.

How many images that you downloaded are for Intel?

```
for f in coreboot*.bin; intel_fw me scan $f; end 2>&1 | grep 'No ME' | wc
    19    133   1235
```

The remaining images can now be used to test `intel_fw`.

### Win-Raid Forum

Many samples can be found via this forum post:
<https://winraid.level1techs.com/t/intel-cs-me-cs-txe-cs-sps-gsc-pmc-pchc-phy-orom-firmware-repositories/30869>
