# RPBC - NDS Program
This is the pairing program that actually generates the tones based on the input on the DS. Technically could run on other DS models, so long as they support nds ROMs.

## Dependencies
This was made using ```libnds```, ```devkitARM```, ```devkitPRO```, ```nds-dev``` and ```grit```. Most (if not all of these) are all able to be installed folowing the [devkitPRO getting started tutorial](https://devkitpro.org/wiki/Getting_Started).

## Compiling
If you have all dependencies, you can generate the `.nds` file by simply typing `make` on the working directory. _(Personally, I had issues generating files when the working directory was on iCloud, so I recommend leaving it in a directory on a physical drive, but who knows, it could just be authorization struggles.)_