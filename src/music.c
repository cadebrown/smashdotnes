#include "music.h"
#include "generated/music_data.h"

#define APU_PULSE1_CTRL  (*(volatile unsigned char*)0x4000)
#define APU_PULSE1_SWEEP (*(volatile unsigned char*)0x4001)
#define APU_PULSE1_LO    (*(volatile unsigned char*)0x4002)
#define APU_PULSE1_HI    (*(volatile unsigned char*)0x4003)
#define APU_PULSE2_CTRL  (*(volatile unsigned char*)0x4004)
#define APU_PULSE2_SWEEP (*(volatile unsigned char*)0x4005)
#define APU_PULSE2_LO    (*(volatile unsigned char*)0x4006)
#define APU_PULSE2_HI    (*(volatile unsigned char*)0x4007)
#define APU_TRI_CTRL     (*(volatile unsigned char*)0x4008)
#define APU_TRI_LO       (*(volatile unsigned char*)0x400A)
#define APU_TRI_HI       (*(volatile unsigned char*)0x400B)
#define APU_NOISE_CTRL   (*(volatile unsigned char*)0x400C)
#define APU_NOISE_PERIOD (*(volatile unsigned char*)0x400E)
#define APU_NOISE_LENGTH (*(volatile unsigned char*)0x400F)
#define APU_STATUS       (*(volatile unsigned char*)0x4015)
#define APU_FRAME        (*(volatile unsigned char*)0x4017)

volatile unsigned char music_debug_scene;
volatile unsigned int music_debug_step;
volatile unsigned char music_debug_last_sfx;

static const MusicStep* music_steps;
static unsigned int music_length;
static unsigned int music_loop;
static unsigned int music_next_step;
static unsigned char music_pulse1_ctrl;
static unsigned char music_pulse2_ctrl;
static unsigned char music_pulse1_hi;
static unsigned char music_pulse2_hi;
static unsigned char music_pulse1_hi_valid;
static unsigned char music_pulse2_hi_valid;
static unsigned char music_pulse1_audible;
static unsigned char music_pulse2_audible;
static unsigned char music_noise_voice;
static unsigned char music_step_frames;
static unsigned char sfx_frames;

static void silence_pulse1(void) {
    APU_PULSE1_CTRL = 0x30;
    music_pulse1_audible = 0;
}

static void silence_pulse2(void) {
    APU_PULSE2_CTRL = 0x30;
    music_pulse2_audible = 0;
}

static void silence_triangle(void) {
    APU_TRI_CTRL = 0x80;
}

static void set_pulse1(unsigned int period) {
    unsigned char high;

    if (period == MUSIC_PERIOD_HOLD) {
        return;
    }
    if (period == MUSIC_PERIOD_REST) {
        silence_pulse1();
        return;
    }
    APU_PULSE1_LO = (unsigned char)period;
    high = (unsigned char)((period >> 8) & 0x07);
    if (!music_pulse1_hi_valid || high != music_pulse1_hi) {
        APU_PULSE1_HI = high;
        music_pulse1_hi = high;
        music_pulse1_hi_valid = 1;
    }
    if (!music_pulse1_audible) {
        APU_PULSE1_CTRL = music_pulse1_ctrl;
        music_pulse1_audible = 1;
    }
}

static void set_pulse2(unsigned int period) {
    unsigned char high;

    if (period == MUSIC_PERIOD_HOLD) {
        return;
    }
    if (period == MUSIC_PERIOD_REST) {
        silence_pulse2();
        return;
    }
    APU_PULSE2_LO = (unsigned char)period;
    high = (unsigned char)((period >> 8) & 0x07);
    if (!music_pulse2_hi_valid || high != music_pulse2_hi) {
        APU_PULSE2_HI = high;
        music_pulse2_hi = high;
        music_pulse2_hi_valid = 1;
    }
    if (!music_pulse2_audible) {
        APU_PULSE2_CTRL = music_pulse2_ctrl;
        music_pulse2_audible = 1;
    }
}

static void set_triangle(unsigned int period) {
    if (period == MUSIC_PERIOD_HOLD) {
        return;
    }
    if (period == MUSIC_PERIOD_REST) {
        silence_triangle();
        return;
    }
    APU_TRI_CTRL = 0xA0;
    APU_TRI_LO = (unsigned char)period;
    APU_TRI_HI = (unsigned char)((period >> 8) & 0x07);
}

static void apply_noise(unsigned char voice) {
    if (voice == 0) {
        APU_NOISE_CTRL = 0x30;
    } else if (voice == 1) {
        APU_NOISE_CTRL = 0x3A;
        APU_NOISE_PERIOD = 0x0D;
        APU_NOISE_LENGTH = 0;
    } else if (voice == 2) {
        APU_NOISE_CTRL = 0x38;
        APU_NOISE_PERIOD = 0x06;
        APU_NOISE_LENGTH = 0;
    } else {
        APU_NOISE_CTRL = 0x34;
        APU_NOISE_PERIOD = 0x02;
        APU_NOISE_LENGTH = 0;
    }
}

static void set_noise(unsigned char voice) {
    if (voice != 4) {
        music_noise_voice = voice;
    }
    if (sfx_frames != 0 || voice == 4) {
        return;
    }
    apply_noise(voice);
}

void __fastcall__ music_set_scene(unsigned char scene) {
    if (scene > MUSIC_SCENE_BATTLE) {
        scene = MUSIC_SCENE_TITLE;
    }
    music_debug_scene = scene;
    if (music_steps != 0) {
        return;
    }

    music_steps = MUSIC_MENU;
    music_length = MUSIC_MENU_LEN;
    music_loop = MUSIC_MENU_LOOP;
    music_pulse1_ctrl = MUSIC_MENU_PULSE1_CTRL;
    music_pulse2_ctrl = MUSIC_MENU_PULSE2_CTRL;

    music_next_step = 0;
    music_debug_step = 0;
    music_step_frames = 0;
    silence_pulse1();
    silence_pulse2();
    silence_triangle();
}

void music_init(void) {
    APU_STATUS = 0;
    APU_FRAME = 0x40;
    APU_PULSE1_SWEEP = 0x08;
    APU_PULSE2_SWEEP = 0x08;
    APU_NOISE_CTRL = 0x30;
    APU_STATUS = 0x0F;
    music_steps = 0;
    sfx_frames = 0;
    music_pulse1_hi_valid = 0;
    music_pulse2_hi_valid = 0;
    music_pulse1_audible = 0;
    music_pulse2_audible = 0;
    music_noise_voice = 0;
    music_debug_scene = 0xFF;
    music_debug_last_sfx = 0;
    music_set_scene(MUSIC_SCENE_TITLE);
}

void __fastcall__ music_sfx(unsigned char cue) {
    music_debug_last_sfx = cue;
    if (cue == MUSIC_SFX_HIT) {
        APU_NOISE_CTRL = 0x3F;
        APU_NOISE_PERIOD = 0x04;
        sfx_frames = 5;
    } else if (cue == MUSIC_SFX_KO) {
        APU_NOISE_CTRL = 0x3F;
        APU_NOISE_PERIOD = 0x0E;
        sfx_frames = 14;
    } else if (cue == MUSIC_SFX_JUMP) {
        APU_NOISE_CTRL = 0x36;
        APU_NOISE_PERIOD = 0x02;
        sfx_frames = 3;
    } else {
        APU_NOISE_CTRL = 0x34;
        APU_NOISE_PERIOD = 0x01;
        sfx_frames = 2;
    }
    APU_NOISE_LENGTH = 0;
}

void music_update(void) {
    const MusicStep* step;

    if (sfx_frames != 0) {
        --sfx_frames;
        if (sfx_frames == 0) {
            apply_noise(music_noise_voice);
        }
    }

    if (music_step_frames != 0) {
        --music_step_frames;
        return;
    }

    step = &music_steps[music_next_step];
    music_debug_step = music_next_step;
    set_pulse1(step->pulse1_period);
    set_pulse2(step->pulse2_period);
    set_triangle(step->triangle_period);
    set_noise(step->noise);
    music_step_frames = (unsigned char)(step->frames - 1);

    ++music_next_step;
    if (music_next_step >= music_length) {
        music_next_step = music_loop;
    }
}
