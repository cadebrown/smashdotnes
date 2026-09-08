#ifndef SMASH_MUSIC_H
#define SMASH_MUSIC_H

#define MUSIC_SCENE_TITLE 0
#define MUSIC_SCENE_LOBBY 1
#define MUSIC_SCENE_BATTLE 2

#define MUSIC_SFX_MENU 1
#define MUSIC_SFX_JUMP 2
#define MUSIC_SFX_HIT 3
#define MUSIC_SFX_KO 4

extern volatile unsigned char music_debug_scene;
extern volatile unsigned int music_debug_step;
extern volatile unsigned char music_debug_last_sfx;

void __fastcall__ music_set_scene(unsigned char scene);
void __fastcall__ music_sfx(unsigned char cue);
void music_init(void);
void music_update(void);

#endif
