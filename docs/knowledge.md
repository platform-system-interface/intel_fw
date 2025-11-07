# Knowledge sources

## Official resources

Intel publishes general information on firmware in their developer portal:
<https://www.intel.com/content/www/us/en/developer/topic-technology/firmware/overview.html>
Note that the public information mainly focuses on host processor firmware.

## Other projects

- [`me_cleaner` wiki](https://github.com/corna/me_cleaner/wiki/)
- [ME Analyzer](https://github.com/platomav/meanalyzer/)
- coreboot
  - [Intel SoC documentation](https://doc.coreboot.org/soc/intel)
  - [Authenticated Code Module](https://doc.coreboot.org/security/intel/acm.html)
  - [`ifdtool`](https://github.com/coreboot/coreboot/tree/main/util/ifdtool)
  - [`intelmetool`](https://github.com/coreboot/coreboot/tree/main/util/intelmetool)
- UEFITool
  - [IFD](https://github.com/LongSoft/UEFITool/blob/new_engine/common/descriptor.h)
  - [ME parser](https://github.com/LongSoft/UEFITool/blob/new_engine/common/meparser.cpp)
  - [FIT parser](https://github.com/LongSoft/UEFITool/blob/new_engine/common/fitparser.cpp)
- Positive Technologies research
  - [TXE MFS exploit PoC](https://github.com/ptresearch/IntelTXE-PoC)
  - [ME disablement](https://github.com/ptresearch/me-disablement)
  - [many other repositories](https://github.com/ptresearch?tab=repositories)

## Forums

People keep [asking in Intel's community forum](
https://community.intel.com/t5/Embedded-Intel-Core-Processors/MFit-Tool-Flash-Image-Tool/m-p/1640753)
for [tools and information around Intel platform firmware](
https://community.intel.com/t5/Embedded-Intel-Core-Processors/Where-to-Download-Flash-Image-Tool-fitc/td-p/249920),
resorting to third-party forums because Intel does not publish what they need or
limits access to necessary resources to certain customers only, notably not end
users.
Here is a short list of places to find useful information and tools.

- Win-Raid (Level1Techs) Forum
  - [(CS)ME firmware and tools](https://winraid.level1techs.com/t/intel-cs-management-engine-drivers-firmware-and-tools-2-15/30719)
  - [(CS)ME etc firmware repositories](https://winraid.level1techs.com/t/intel-cs-me-cs-txe-cs-sps-gsc-pmc-pchc-phy-orom-firmware-repositories/30869)
  - [ME Analyzer](https://winraid.level1techs.com/t/me-analyzer-intel-engine-firmware-analysis-tool-discussion/30876)
  - [EFFS in ME Gen 2](https://winraid.level1techs.com/t/parsing-effs-partition/34609)
- [Badcaps Forum](https://www.badcaps.net/forum/troubleshooting-hardware-devices-and-electronics-theory/troubleshooting-desktop-motherboards-graphics-cards-and-pc-peripherals/105308-fit-csme-tool-flash-image-tool-trusted-download-location)
- [Indiafix Forum](https://www.indiafix.in/2024/09/download-intel-flash-image-tool-fitc.html?m=1)
- [Vinafix Forum](https://vinafix.com/tags/flash-image-tool/)
- [AliSaler](https://www.alisaler.com/intel-me-system-tools-v11-6-r8-flash-image-tool-download/)
- [mostav02's notes on FPT and removals](https://github.com/mostav02/Remove_IntelME_FPT)

### Extraction

For older FIT tools, use `binwalk` to extract resources and then `grep` for
`LayoutEntry` to find descriptions of straps, which include the HAP bit, e.g.:

```
<LayoutEntry name="PCH_Strap_CSME_CSE_HAP_Mode" type="bitfield32"
    value="0x0" offset="0x68" bitfield_high="16" bitfield_low="16" />
```

Note that later generation XML files just call the HAP bit "reserved".
Educated guessing by looking at neighboring bits will help you to locate it.

Newer FIT tools (e.g., v18) contain Python code that can be extracted with
[pyinstxtractor](https://github.com/extremecoders-re/pyinstxtractor). Relevant
code is in the `plugins/` directory:

```sh
pyinstxtractor.py ../mfit_18.exe
ls -l tools.exe_extracted/plugins/
```
