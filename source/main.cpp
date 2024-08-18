#include <nds.h>

#include <stdio.h>

// git adds a nice header we can include to access the data
// this has the same name as the image
#include "flowers.h"

u16 freqs[] = {240, 260, 280, 300, 320, 340, 360, 380, 400, 420, 440, 460, 480, 500};
int wave = 0;
int touchWaveL = 0, touchWaveR = 0;

int main(void)
{
	touchPosition touchXY;

    // set the mode for 2 text layers and two extended background layers
	videoSetMode(MODE_5_2D);
    vramSetBankA(VRAM_A_MAIN_BG_0x06000000);

	consoleDemoInit();
	soundEnable();

	iprintf("\x1b[35;1mRPBC-Rose'sPitch-BasedController");
	iprintf("  \x1b[35mgithub.com/rosieparfait/rpbc\x1b[39m\n");
	

	int bg3 = bgInit(3, BgType_Bmp8, BgSize_B8_256x256, 0,0);

	dmaCopy(flowersBitmap, bgGetGfxPtr(bg3), 256*256);
	dmaCopy(flowersPal, BG_PALETTE, 256*2);

	iprintf("\x1b[23;0Hv0.1.1\x1b[39;49m");

	while(1) {
	
		swiWaitForVBlank();
		scanKeys();
		int keys = keysHeld();

		touchRead(&touchXY);

		// Reposition cursor to original location
		iprintf("\x1b[4;0H\x1b[39;49m");

		// Print keys as binary, digit by digit, and play wavelength if pressed
		bool pressingButton = false;

		for (int i = 31; i >= 0; i--) {
			int state = (keys & (1 << i)) >> i;
			iprintf("%d", state);
			// if pressing buttons but not touchscreen
			if (i < 12 && touchWaveL == 0) {
				if (state) {
					if (wave != 0) {
						soundSetFreq(wave, freqs[i] << 3);
					} else {
						wave = soundPlayPSG(DutyCycle_12, freqs[i] << 3, 80, 64);
					}
					pressingButton = true;
				}
			} 
			// if touching screen
			else if (i == 12 && state) {
				u16 x_freq = 550 + 3*touchXY.px;
				u16 y_freq = 550 + 3*touchXY.py;

				if (touchWaveL != 0) {
					soundSetFreq(touchWaveL, x_freq << 3);
					soundSetFreq(touchWaveR, y_freq << 3);
				} else {
					touchWaveL = soundPlayPSG(DutyCycle_25, x_freq << 3, 80, 0);
					touchWaveR = soundPlayPSG(DutyCycle_37, y_freq << 3, 80, 127);
				}
				pressingButton = true;
			}
		}
		if (!pressingButton) { soundKill(wave); wave = 0; soundKill(touchWaveL); touchWaveL = 0; soundKill(touchWaveR); touchWaveR = 0;}

		// Print legend
		iprintf("                   tYXLRdulrSsBA\n");

		iprintf("Touch x = %04d, %04d\n", touchXY.rawx, touchXY.px);
		iprintf("Touch y = %04d, %04d\n", touchXY.rawy, touchXY.py);	
		iprintf("\n\n\x1b[31mLegend (for ambiguous symbols):\nt: Touch Screen\nd: D-Pad Down\nu: D-Pad up\nl: D-Pad Left\nr: D-Pad Right\nS: Start\ns: Select");
	
	}

	return 0;
}
