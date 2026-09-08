#include <nes.h>
#include "generated/title_logo.h"
#include "music.h"

/*
 * Smash prototype
 * - 2-player versus, 1P vs CPU, and CPU vs CPU
 * - All control flows through the same input path
 * - Uses fixed-point-free integer physics for NES friendliness
 */

typedef unsigned char  u8;
typedef signed char    s8;
typedef unsigned int   u16;
typedef signed int     s16;

#include "generated/roster_runtime.h"

#define BTN_A      0x01
#define BTN_B      0x02
#define BTN_SELECT 0x04
#define BTN_START  0x08
#define BTN_UP     0x10
#define BTN_DOWN   0x20
#define BTN_LEFT   0x40
#define BTN_RIGHT  0x80

#define MODE_2P       0
#define MODE_1P_CPU   1
#define MODE_CPU_CPU  2

#define STATE_TITLE  0
#define STATE_LOBBY  1
#define STATE_BATTLE 2

#define CHAR_MARIO   0
#define CHAR_SIMON   1
#define CHAR_TOMMY  2
#define CHAR_RIZER    3
#define CHAR_COUNT   ROSTER_CHARACTER_COUNT

#define PROJECTILE_NONE     0
#define PROJECTILE_FIREBALL 1
#define PROJECTILE_AXE      2
#define PROJECTILE_BULLET   3

#define MAX_PROJECTILES 4

#define PLATFORM_Y      176
#define PLATFORM_LEFT   32
#define PLATFORM_WIDTH  192
#define PLATFORM_RIGHT  (PLATFORM_LEFT + PLATFORM_WIDTH)
#define PLATFORM_TILE_X (PLATFORM_LEFT / 8)
#define PLATFORM_TILE_Y (PLATFORM_Y / 8)
#define PLATFORM_TILE_W (PLATFORM_WIDTH / 8)

/* OAM Y is top-1 on NES; 143 visually lands 16x32 fighter feet on platform at y=176. */
#define GROUND_Y        143

#define BLAST_LEFT      -24
#define BLAST_RIGHT     264
#define BLAST_BOTTOM    240

#define FIGHTER_W 16
#define FIGHTER_H 32

#define TILE_BLANK      0
#define TILE_GROUND     1
#define TILE_DASH       2
#define TILE_COLON      3
#define TILE_0          4
#define TILE_ARROW_BG   14
#define TILE_PLUS       15
#define TILE_CLOUD_L    18
#define TILE_CLOUD_M    19
#define TILE_CLOUD_R    20
#define TILE_UI_HLINE   21
#define TILE_UI_VLINE   22
#define TILE_UI_CORNER  23
#define TILE_PERCENT    24
#define TILE_ICON_MARIO 26
#define TILE_ICON_SIMON 27
#define TILE_ICON_TOMMY 28
#define TILE_ICON_RIZER 29
#define TILE_HILL_L     30
#define TILE_HILL_R     31
#define TILE_A          32
#define TILE_PLATFORM_A 16
#define TILE_PLATFORM_B 17

#define SPR_FIREBALL_FRAME_0 ROSTER_RUNTIME_TILE_COUNT
#define SPR_AXE_FRAME_0      (SPR_FIREBALL_FRAME_0 + 2)
#define SPR_SIMON_WHIP_0     (SPR_AXE_FRAME_0 + 2)
#define SPR_SIMON_WHIP_MID   (SPR_SIMON_WHIP_0 + 1)
#define SPR_BULLET_REAL      (SPR_SIMON_WHIP_0 + 2)
#define SPR_HIT              (SPR_BULLET_REAL + 1)
#define SPR_CURSOR           SPR_HIT

#define PPUCTRL_MAIN  0x08  /* sprite pattern table = $1000, bg = $0000 */
#define PPUMASK_MAIN  0x18  /* show bg + sprites, clip left 8px (PPUMASK bits 1/2) */

#define PPUCTRL  (*(volatile u8*)0x2000)
#define PPUMASK  (*(volatile u8*)0x2001)
#define PPUSTATUS (*(volatile u8*)0x2002)
#define OAMADDR  (*(volatile u8*)0x2003)
#define OAMDATA  (*(volatile u8*)0x2004)
#define PPUSCROLL (*(volatile u8*)0x2005)
#define PPUADDR  (*(volatile u8*)0x2006)
#define PPUDATA  (*(volatile u8*)0x2007)
#define OAMDMA   (*(volatile u8*)0x4014)
#define JOY1     (*(volatile u8*)0x4016)
#define JOY2     (*(volatile u8*)0x4017)

#define OAM ((volatile u8*)0x0200)
/*
 * Headless automation export surface (CPU RAM, $06E0 base):
 *   0:'S' 1:'N' 2:state 3:mode 4:frame 5:pad1 6:pad2 7:projectiles
 *   8..9:p1.x 10..11:p1.y 12:p1.damage 13:p1.ko
 *   14..15:p2.x 16..17:p2.y 18:p2.damage 19:p2.ko
 *   20:music scene, 21..22:music event, 23:last sound effect
 *   30:p1.render_frame (low nibble), p2.render_frame (high nibble)
 *   31:OAM sprites (low six bits), active hit effects (high two bits)
 * Keep this layout in sync with docs/architecture.md and emulator Lua scripts.
 */
#define TEST_RAM ((volatile u8*)0x06E0)

typedef struct Fighter Fighter;

typedef struct {
    u8 active;
    u8 owner;
    u8 kind;
    u8 palette;
    u8 ttl;
    s16 x;
    s16 y;
    s8 vx;
    s8 vy;
} Projectile;

typedef struct {
    u8 timer;
    u8 palette;
    s16 x;
    s16 y;
} HitEffect;

struct Fighter {
    u8 character;
    u8 is_cpu;
    u8 facing;
    u8 on_ground;
    u8 jumps_used;
    u8 control;
    u8 prev_control;
    u8 attack_timer;
    u8 special_timer;
    u8 attack_hit;
    u8 special_hit;
    u8 damage;
    u8 ko_count;
    s8 hitstun;
    s8 knock_vx;
    s8 vy;
    s16 x;
    s16 y;
};

static const u8 palette[32] = {
    /* Background palettes */
    0x0F, 0x16, 0x27, 0x30, /* red/orange/white  (P1) */
    0x0F, 0x1A, 0x2A, 0x30, /* green/lime/white  (P2) */
    0x0F, 0x30, 0x27, 0x17, /* white + SMB1-style brown shades (AI + platform) */
    0x0F, 0x30, 0x20, 0x30, /* white/gray/white (VS/neutral) */
    /* Sprite palettes */
    0x0F, 0x0F, 0x16, 0x27, /* Mario SMB3: black outline, red, skin */
    0x0F, 0x06, 0x27, 0x37, /* Simon CV1-ish: dark red, orange, light skin */
    0x0F, 0x0F, 0x27, 0x30, /* Tommy KF-ish: black, orange, white */
    0x0F, 0x0F, 0x16, 0x30  /* Rizer Contra-ish: black, red, white */
};

static const char* const character_names[CHAR_COUNT] = {
    "MARIO", "SIMON", "TOMMY", "RIZER"
};

static const char* const mode_names[3] = {
    "P1 VS P2",
    "P1 VS AI",
    "AI VS AI"
};

#define TITLE_MENU_WIDTH_TILES 8
#define TITLE_MENU_X ((32 - TITLE_MENU_WIDTH_TILES) / 2)
#define TITLE_MENU_Y 16
#define TITLE_MENU_STEP 2
#define TITLE_CURSOR_TILE_X (TITLE_MENU_X - 2)

#define TITLE_PAL_1P 0
#define TITLE_PAL_2P 1
#define TITLE_PAL_AI 2
#define TITLE_PAL_VS 3

#define TITLE_VARIANT_WORLD       0
#define TITLE_VARIANT_CHALLENGERS 1
#define TITLE_VARIANT_EMBLEM      2

#ifndef TITLE_VARIANT
#define TITLE_VARIANT TITLE_VARIANT_WORLD
#endif

#if TITLE_VARIANT < TITLE_VARIANT_WORLD || TITLE_VARIANT > TITLE_VARIANT_EMBLEM
#error Invalid TITLE_VARIANT
#endif

#define LOBBY_GRID_X 8
#define LOBBY_GRID_Y 6
#define LOBBY_CELL_W 8
#define LOBBY_CELL_H 8
#define LOBBY_GRID_W (LOBBY_CELL_W * 2)
#define LOBBY_GRID_H (LOBBY_CELL_H * 2)

static const u8 lobby_card_x[CHAR_COUNT] = {
    LOBBY_GRID_X,
    LOBBY_GRID_X + LOBBY_CELL_W,
    LOBBY_GRID_X,
    LOBBY_GRID_X + LOBBY_CELL_W
};

static const u8 lobby_card_y[CHAR_COUNT] = {
    LOBBY_GRID_Y,
    LOBBY_GRID_Y,
    LOBBY_GRID_Y + LOBBY_CELL_H,
    LOBBY_GRID_Y + LOBBY_CELL_H
};

static const u8 char_palette_id[CHAR_COUNT] = {
    0, 1, 2, 3
};

static const u8 jab_range[CHAR_COUNT]  = {10, 20, 12, 14};
static const u8 jab_damage[CHAR_COUNT] = { 6,  7,  6,  6};

static u8 game_state;
static u8 game_mode;
static u8 mode_cursor;
static u8 p1_char;
static u8 p2_char;
static u8 frame_counter;
static u8 battle_frame_counter;

static u8 pad1;
static u8 pad2;
static u8 pad1_prev;
static u8 pad2_prev;

static Fighter fighters[2];
static Projectile projectiles[MAX_PROJECTILES];
static HitEffect hit_effects[2];

static u8 oam_used;
static u8 oam_previous_used;
static u8 title_attr[64];
static u8 battle_hud_prev_damage[2];
static char battle_hud_text[2][5];
static u8 battle_hud_dirty[2];
static u8 title_cursor_rendered;
static u8 fighter_render_frame[2];
static u8 active_projectile_count;

static void oam_put(s16 x, s16 y, u8 tile, u8 attr);
static void draw_character_icon(u8 character, u8 frame, s16 x, s16 y, u8 facing_right);

static u8 absdiff8(s16 a, s16 b) {
    s16 d = a - b;
    if (d < 0) {
        d = -d;
    }
    return (u8)d;
}

static u8 rising_edge(u8 now, u8 prev, u8 mask) {
    return (u8)((now & mask) && !(prev & mask));
}

static u8 read_controller(u8 port) {
    u8 i;
    u8 value = 0;

    JOY1 = 1;
    JOY1 = 0;

    for (i = 0; i < 8; ++i) {
        value >>= 1;
        if (port == 0) {
            if (JOY1 & 1) {
                value |= 0x80;
            }
        } else {
            if (JOY2 & 1) {
                value |= 0x80;
            }
        }
    }

    /* Final layout: A,B,Select,Start,Up,Down,Left,Right */
    return value;
}

static void ppu_set_addr(u16 addr) {
    (void)PPUSTATUS;
    PPUADDR = (u8)(addr >> 8);
    PPUADDR = (u8)(addr & 0xFF);
}

static void ppu_off(void) {
    PPUMASK = 0x00;
}

static void ppu_on(void) {
    PPUCTRL = PPUCTRL_MAIN;
    PPUSCROLL = 0;
    PPUSCROLL = 0;
    PPUMASK = PPUMASK_MAIN;
}

/* Keep render scroll stable after any live PPUADDR/PPUDATA writes.
 * NESdev reference: PPUADDR/PPUSCROLL share the internal t/v latch state. */
static void ppu_restore_scroll_state(void) {
    PPUCTRL = PPUCTRL_MAIN;
    PPUSCROLL = 0;
    PPUSCROLL = 0;
}

/* waitvsync() in cc65 only waits for vblank-set; if called while already in
 * vblank it can return immediately. We need a clean 0->1 transition so all
 * PPUDATA writes stay inside a known vblank window. */
static void wait_vblank_strict(void) {
    while (PPUSTATUS & 0x80) {
    }
    while (!(PPUSTATUS & 0x80)) {
    }
}

static void ppu_load_palette(void) {
    u8 i;
    ppu_set_addr(0x3F00);
    for (i = 0; i < 32; ++i) {
        PPUDATA = palette[i];
    }
}

static void ppu_set_universal_background(u8 color) {
    ppu_set_addr(0x3F00);
    PPUDATA = color;
}

static void clear_nametable(u8 tile) {
    u16 i;
    ppu_set_addr(0x2000);
    for (i = 0; i < 1024; ++i) {
        PPUDATA = tile;
    }
}

static u8 char_to_tile(char c) {
    if (c >= 'A' && c <= 'Z') {
        return (u8)(TILE_A + (u8)(c - 'A'));
    }
    if (c >= '0' && c <= '9') {
        return (u8)(TILE_0 + (u8)(c - '0'));
    }
    if (c == ' ') {
        return TILE_BLANK;
    }
    if (c == '-') {
        return TILE_DASH;
    }
    if (c == ':') {
        return TILE_COLON;
    }
    if (c == '>') {
        return TILE_ARROW_BG;
    }
    if (c == '+') {
        return TILE_PLUS;
    }
    if (c == '%') {
        return TILE_PERCENT;
    }
    return TILE_BLANK;
}

static void draw_text(u8 x, u8 y, const char* text) {
    u16 addr = (u16)0x2000 + (((u16)y) << 5) + x;
    ppu_set_addr(addr);
    while (*text) {
        PPUDATA = char_to_tile(*text);
        ++text;
    }
}

static void draw_tile(u8 x, u8 y, u8 tile) {
    u16 addr = (u16)0x2000 + (((u16)y) << 5) + x;
    ppu_set_addr(addr);
    PPUDATA = tile;
}

static u8 text_len(const char* text) {
    u8 n = 0;
    while (text[n]) {
        ++n;
    }
    return n;
}

static void draw_text_centered(u8 x, u8 y, u8 width, const char* text) {
    u8 len = text_len(text);
    if (len >= width) {
        draw_text(x, y, text);
    } else {
        draw_text((u8)(x + ((width - len) >> 1)), y, text);
    }
}

static void format_damage_text(u8 value, char* out) {
    u8 hundreds = (u8)(value / 100);
    u8 tens = (u8)((value / 10) % 10);
    u8 ones = (u8)(value % 10);

    out[0] = ' ';
    out[1] = ' ';
    out[2] = ' ';
    out[3] = '%';
    out[4] = '\0';

    if (value >= 100) {
        out[0] = (char)('0' + hundreds);
        out[1] = (char)('0' + tens);
        out[2] = (char)('0' + ones);
    } else if (value >= 10) {
        out[1] = (char)('0' + tens);
        out[2] = (char)('0' + ones);
    } else {
        out[2] = (char)('0' + ones);
    }
}

static void draw_battle_damage_text_bg(u8 player_idx, u8 value) {
    u8 x = player_idx ? 21 : 5;
    (void)value;
    draw_text(x, 4, battle_hud_text[player_idx]);
}

static void refresh_battle_damage_bg(void) {
    u8 wrote = 0;

    if (game_state != STATE_BATTLE) {
        return;
    }

    if (battle_hud_dirty[0]) {
        draw_battle_damage_text_bg(0, fighters[0].damage);
        battle_hud_dirty[0] = 0;
        wrote = 1;
    }
    if (battle_hud_dirty[1]) {
        draw_battle_damage_text_bg(1, fighters[1].damage);
        battle_hud_dirty[1] = 0;
        wrote = 1;
    }

    if (wrote) {
        ppu_restore_scroll_state();
    }
}

static void refresh_title_cursor_bg(void) {
    u8 row_y;

    if (game_state != STATE_TITLE) {
        return;
    }
    if (title_cursor_rendered == mode_cursor) {
        return;
    }

    if (title_cursor_rendered < 3) {
        row_y = (u8)(TITLE_MENU_Y + (title_cursor_rendered * TITLE_MENU_STEP));
        draw_tile(TITLE_CURSOR_TILE_X, row_y, TILE_BLANK);
    }

    row_y = (u8)(TITLE_MENU_Y + (mode_cursor * TITLE_MENU_STEP));
    draw_tile(TITLE_CURSOR_TILE_X, row_y, TILE_ARROW_BG);
    title_cursor_rendered = mode_cursor;
    ppu_restore_scroll_state();
}

static void draw_box(u8 x, u8 y, u8 width, u8 height) {
    u8 i;

    if (width < 2 || height < 2) {
        return;
    }

    draw_tile(x, y, TILE_UI_CORNER);
    draw_tile((u8)(x + width - 1), y, TILE_UI_CORNER);
    draw_tile(x, (u8)(y + height - 1), TILE_UI_CORNER);
    draw_tile((u8)(x + width - 1), (u8)(y + height - 1), TILE_UI_CORNER);

    for (i = 1; i + 1 < width; ++i) {
        draw_tile((u8)(x + i), y, TILE_UI_HLINE);
        draw_tile((u8)(x + i), (u8)(y + height - 1), TILE_UI_HLINE);
    }

    for (i = 1; i + 1 < height; ++i) {
        draw_tile(x, (u8)(y + i), TILE_UI_VLINE);
        draw_tile((u8)(x + width - 1), (u8)(y + i), TILE_UI_VLINE);
    }
}

/*
 * Title attributes are composed in RAM and then committed in one VRAM pass.
 * References:
 * - https://www.nesdev.org/wiki/PPU_attribute_tables
 * - https://www.nesdev.org/wiki/PPU_nametables
 */
static void title_attr_clear(void) {
    u8 i;
    for (i = 0; i < 64; ++i) {
        title_attr[i] = 0;
    }
}

static void title_attr_set_tile_palette(u8 tile_x, u8 tile_y, u8 palette_id) {
    u8 attr_x = (u8)(tile_x >> 2);
    u8 attr_y = (u8)(tile_y >> 2);
    u8 index = (u8)(attr_y * 8 + attr_x);
    u8 shift = (u8)((tile_x & 0x02) | ((tile_y & 0x02) << 1));
    u8 mask = (u8)(0x03u << shift);

    title_attr[index] = (u8)((title_attr[index] & (u8)(~mask)) | ((palette_id & 0x03u) << shift));
}

static void title_attr_fill_rect(u8 tile_x, u8 tile_y, u8 width, u8 height, u8 palette_id) {
    u8 y;
    u8 x;
    for (y = 0; y < height; ++y) {
        for (x = 0; x < width; ++x) {
            title_attr_set_tile_palette((u8)(tile_x + x), (u8)(tile_y + y), palette_id);
        }
    }
}

static void title_attr_commit(void) {
    u8 i;
    ppu_set_addr(0x23C0);
    for (i = 0; i < 64; ++i) {
        PPUDATA = title_attr[i];
    }
}

static void draw_title_logo_letters(void) {
    u8 letter;
    u8 row;
    u8 col;

    for (letter = 0; letter < TITLE_LOGO_LETTER_COUNT; ++letter) {
        u8 tile_base = (u8)(TITLE_LOGO_TILE_BASE + (letter * TITLE_LOGO_TILE_STRIDE));
        u8 x_start = title_letter_pos_x[letter];
        u8 draw_w = title_letter_draw_w[letter];

        for (row = 0; row < TITLE_LOGO_LETTER_H; ++row) {
            for (col = 0; col < draw_w; ++col) {
                u8 tile = (u8)(tile_base + (row * TITLE_LOGO_LETTER_W) + col);
                draw_tile((u8)(x_start + col), (u8)(TITLE_LOGO_START_Y + row), tile);
            }
        }
    }
}

#if TITLE_VARIANT == TITLE_VARIANT_WORLD
static const u8 title_variant_fx[] = {0, 1, 4, 5};
#elif TITLE_VARIANT == TITLE_VARIANT_CHALLENGERS
static const u8 title_variant_fx[] = {0, 1, 2, 3, 6, 7, 8, 9, 14, 15, 29, 30};
#else
static const u8 title_variant_fx[] = {2, 3};
#endif

#define TITLE_VARIANT_FX_COUNT (sizeof(title_variant_fx) / sizeof(title_variant_fx[0]))

static void draw_title_fx_field(void) {
    u8 i;

    for (i = 0; i < TITLE_VARIANT_FX_COUNT; ++i) {
        u8 letter = title_variant_fx[i];
        draw_tile(
            title_fx_pos[letter][0],
            title_fx_pos[letter][1],
            (u8)(TITLE_FX_TILE_BASE + title_fx_tile[letter])
        );
    }
}

#if TITLE_VARIANT == TITLE_VARIANT_WORLD
static void draw_title_scene(void) {
    u8 x;

    ppu_set_universal_background(0x21);

    draw_tile(2, 2, TILE_CLOUD_L);
    draw_tile(3, 2, TILE_CLOUD_M);
    draw_tile(4, 2, TILE_CLOUD_R);
    draw_tile(27, 4, TILE_CLOUD_L);
    draw_tile(28, 4, TILE_CLOUD_M);
    draw_tile(29, 4, TILE_CLOUD_R);

    draw_title_logo_letters();
    draw_title_fx_field();
    draw_box(9, 13, 14, 10);
    draw_text(11, 14, "SELECT MODE");

    for (x = 0; x < 4; ++x) {
        draw_tile((u8)(2 + x), 23, (u8)((x & 1) ? TILE_HILL_R : TILE_HILL_L));
        draw_tile((u8)(26 + x), 23, (u8)((x & 1) ? TILE_HILL_R : TILE_HILL_L));
    }
    for (x = 0; x < 32; ++x) {
        draw_tile(x, 25, (u8)((x & 1) ? TILE_PLATFORM_B : TILE_PLATFORM_A));
        draw_tile(x, 26, (u8)((x & 1) ? TILE_PLATFORM_A : TILE_PLATFORM_B));
    }
    draw_text(10, 28, "NES ALL STARS");
}
#elif TITLE_VARIANT == TITLE_VARIANT_CHALLENGERS
static void draw_title_scene(void) {
    ppu_set_universal_background(0x0F);
    draw_title_logo_letters();
    draw_title_fx_field();
    draw_box(8, 13, 16, 10);
    draw_text(9, 14, "WORLDS COLLIDE");
}
#else
static void draw_title_scene(void) {
    u8 i;

    ppu_set_universal_background(0x0F);
    for (i = 1; i < 31; ++i) {
        draw_tile(i, 3, TILE_UI_HLINE);
        draw_tile(i, 25, TILE_UI_HLINE);
    }
    for (i = 0; i < 29; ++i) {
        if (i < 13 || i > 22) {
            draw_tile(15, i, TILE_UI_VLINE);
        }
    }
    draw_tile(15, 3, TILE_PLUS);
    draw_tile(15, 25, TILE_PLUS);

    draw_title_logo_letters();
    draw_title_fx_field();
    draw_box(8, 13, 16, 10);
    draw_text(10, 14, "MELEE EDITION");

    draw_tile(3, 2, TITLE_FX_TILE_BASE + title_fx_tile[4]);
    draw_tile(28, 2, TITLE_FX_TILE_BASE + title_fx_tile[4]);
    draw_tile(3, 26, TITLE_FX_TILE_BASE + title_fx_tile[6]);
    draw_tile(28, 26, TITLE_FX_TILE_BASE + title_fx_tile[6]);
    draw_tile(6, 15, TITLE_FX_TILE_BASE + title_fx_tile[1]);
    draw_tile(25, 15, TITLE_FX_TILE_BASE + title_fx_tile[1]);
}
#endif

static void draw_title_palette_zones(void) {
    u8 letter;

    title_attr_clear();

#if TITLE_VARIANT == TITLE_VARIANT_EMBLEM
    title_attr_fill_rect(0, 2, 32, 2, TITLE_PAL_1P);
    title_attr_fill_rect(14, 0, 4, 30, TITLE_PAL_1P);
    title_attr_fill_rect(0, 24, 32, 4, TITLE_PAL_1P);
#endif
    for (letter = 0; letter < TITLE_LOGO_LETTER_COUNT; ++letter) {
        u8 x_start = title_letter_pos_x[letter];
        u8 draw_w = title_letter_draw_w[letter];
        title_attr_fill_rect(x_start, TITLE_LOGO_START_Y, draw_w, TITLE_LOGO_LETTER_H, title_letter_palette[letter]);
    }

    for (letter = 0; letter < TITLE_VARIANT_FX_COUNT; ++letter) {
        u8 index = title_variant_fx[letter];
        title_attr_set_tile_palette(
            title_fx_pos[index][0],
            title_fx_pos[index][1],
            title_fx_palette[index]
        );
    }

#if TITLE_VARIANT == TITLE_VARIANT_WORLD
    title_attr_fill_rect(0, 0, 32, 6, TITLE_PAL_VS);
    title_attr_fill_rect(9, 13, 14, 10, TITLE_PAL_VS);
    title_attr_fill_rect(0, 22, 32, 3, TITLE_PAL_2P);
    title_attr_fill_rect(0, 25, 32, 5, TITLE_PAL_AI);
#elif TITLE_VARIANT == TITLE_VARIANT_CHALLENGERS
    title_attr_fill_rect(8, 13, 16, 10, TITLE_PAL_VS);
#else
    title_attr_fill_rect(8, 13, 16, 10, TITLE_PAL_VS);
#endif
}

static void draw_title_menu_options(void) {
    u8 row0 = TITLE_MENU_Y + (TITLE_MENU_STEP * 0);
    u8 row1 = TITLE_MENU_Y + (TITLE_MENU_STEP * 1);
    u8 row2 = TITLE_MENU_Y + (TITLE_MENU_STEP * 2);

    /* Width is always 8 tiles; x=12 centers "?? VS ??" and keeps VS at x=15. */
    draw_text(TITLE_MENU_X + 0, row0, "P1");
    draw_text(TITLE_MENU_X + 3, row0, "VS");
    draw_text(TITLE_MENU_X + 6, row0, "P2");

    draw_text(TITLE_MENU_X + 0, row1, "P1");
    draw_text(TITLE_MENU_X + 3, row1, "VS");
    draw_text(TITLE_MENU_X + 6, row1, "AI");

    draw_text(TITLE_MENU_X + 0, row2, "AI");
    draw_text(TITLE_MENU_X + 3, row2, "VS");
    draw_text(TITLE_MENU_X + 6, row2, "AI");

    title_attr_fill_rect(TITLE_MENU_X + 0, row0, 2, 1, TITLE_PAL_1P);
    title_attr_fill_rect(TITLE_MENU_X + 3, row0, 2, 1, TITLE_PAL_VS);
    title_attr_fill_rect(TITLE_MENU_X + 6, row0, 2, 1, TITLE_PAL_2P);

    title_attr_fill_rect(TITLE_MENU_X + 0, row1, 2, 1, TITLE_PAL_1P);
    title_attr_fill_rect(TITLE_MENU_X + 3, row1, 2, 1, TITLE_PAL_VS);
    title_attr_fill_rect(TITLE_MENU_X + 6, row1, 2, 1, TITLE_PAL_AI);

    title_attr_fill_rect(TITLE_MENU_X + 0, row2, 2, 1, TITLE_PAL_AI);
    title_attr_fill_rect(TITLE_MENU_X + 3, row2, 2, 1, TITLE_PAL_VS);
    title_attr_fill_rect(TITLE_MENU_X + 6, row2, 2, 1, TITLE_PAL_AI);
    title_attr_fill_rect(TITLE_CURSOR_TILE_X, row0, 1, 1, TITLE_PAL_1P);
    title_attr_fill_rect(TITLE_CURSOR_TILE_X, row1, 1, 1, TITLE_PAL_1P);
    title_attr_fill_rect(TITLE_CURSOR_TILE_X, row2, 1, 1, TITLE_PAL_1P);

    title_attr_commit();
}

static void draw_lobby_palette_zones(void) {
    u8 i;

    title_attr_clear();

    title_attr_fill_rect(0, 0, 32, 6, TITLE_PAL_VS);

    for (i = 0; i < CHAR_COUNT; ++i) {
        title_attr_fill_rect(
            lobby_card_x[i],
            lobby_card_y[i],
            LOBBY_CELL_W,
            LOBBY_CELL_H,
            char_palette_id[i]
        );
    }

    title_attr_commit();
}

static void draw_lobby_layout(void) {
    u8 i;
    u8 gx = LOBBY_GRID_X;
    u8 gy = LOBBY_GRID_Y;

    draw_text(13, 2, "LOBBY");
    draw_text(4, 4, "MODE:");
    draw_text(10, 4, mode_names[game_mode]);

    /* Large 2x2 connected roster grid with 64x48 pixel cells. */
    draw_box(gx, gy, LOBBY_GRID_W, LOBBY_GRID_H);
    for (i = 1; i + 1 < LOBBY_GRID_H; ++i) {
        draw_tile((u8)(gx + LOBBY_CELL_W), (u8)(gy + i), TILE_UI_VLINE);
    }
    for (i = 1; i + 1 < LOBBY_GRID_W; ++i) {
        draw_tile((u8)(gx + i), (u8)(gy + LOBBY_CELL_H), TILE_UI_HLINE);
    }
    draw_tile((u8)(gx + LOBBY_CELL_W), (u8)(gy + LOBBY_CELL_H), TILE_UI_CORNER);

    for (i = 0; i < CHAR_COUNT; ++i) {
        draw_text_centered(
            (u8)(lobby_card_x[i] + 1),
            (u8)(lobby_card_y[i] + 1),
            (u8)(LOBBY_CELL_W - 2),
            character_names[i]
        );
    }

    draw_text(9, 27, "START TO FIGHT");
}

static void draw_battle_palette_zones(void) {
    title_attr_clear();

    title_attr_fill_rect(0, 0, 32, 30, TITLE_PAL_VS);
    title_attr_fill_rect(3, 2, 12, 4, TITLE_PAL_1P);
    title_attr_fill_rect(17, 2, 12, 4, TITLE_PAL_2P);
    title_attr_fill_rect(3, 10, 3, 1, TITLE_PAL_VS);
    title_attr_fill_rect(13, 13, 3, 1, TITLE_PAL_VS);
    title_attr_fill_rect(23, 11, 3, 1, TITLE_PAL_VS);
    title_attr_fill_rect(1, 20, 4, 2, TITLE_PAL_2P);
    title_attr_fill_rect(11, 20, 4, 2, TITLE_PAL_2P);
    title_attr_fill_rect(27, 20, 4, 2, TITLE_PAL_2P);
    title_attr_fill_rect(PLATFORM_TILE_X, PLATFORM_TILE_Y, PLATFORM_TILE_W, 4, TITLE_PAL_AI);

    title_attr_commit();
}

static void draw_battle_layout(void) {
    u8 i;

    for (i = 2; i < 30; ++i) {
        draw_tile(i, 6, TILE_UI_HLINE);
    }
    draw_tile(16, 6, TILE_PLUS);
    draw_text(12, 8, "ROUND 01");

    draw_tile(3, 10, TILE_CLOUD_L);
    draw_tile(4, 10, TILE_CLOUD_M);
    draw_tile(5, 10, TILE_CLOUD_R);
    draw_tile(13, 13, TILE_CLOUD_L);
    draw_tile(14, 13, TILE_CLOUD_M);
    draw_tile(15, 13, TILE_CLOUD_R);
    draw_tile(23, 11, TILE_CLOUD_L);
    draw_tile(24, 11, TILE_CLOUD_M);
    draw_tile(25, 11, TILE_CLOUD_R);

    draw_tile(1, 21, TILE_HILL_L);
    draw_tile(2, 21, TILE_HILL_R);
    draw_tile(3, 21, TILE_HILL_L);
    draw_tile(4, 21, TILE_HILL_R);
    draw_tile(11, 21, TILE_HILL_L);
    draw_tile(12, 21, TILE_HILL_R);
    draw_tile(13, 21, TILE_HILL_L);
    draw_tile(14, 21, TILE_HILL_R);
    draw_tile(27, 21, TILE_HILL_L);
    draw_tile(28, 21, TILE_HILL_R);
    draw_tile(29, 21, TILE_HILL_L);
    draw_tile(30, 21, TILE_HILL_R);
}

static void init_oam(void) {
    u8 i;
    u8 o = 0;
    for (i = 0; i < 64; ++i) {
        /* Hide unused sprites below visible scanlines to avoid wrap artifacts. */
        OAM[o] = 0xF0;
        o = (u8)(o + 4);
    }
    oam_used = 0;
    oam_previous_used = 0;
}

static void begin_oam(void) {
    oam_used = 0;
}

static void finish_oam(void) {
    u8 i = oam_used;

    while (i < oam_previous_used) {
        OAM[i << 2] = 0xF0;
        ++i;
    }
    oam_previous_used = oam_used;
}

static void oam_put(s16 x, s16 y, u8 tile, u8 attr) {
    u8 base;

    if (oam_used >= 64) {
        return;
    }

    /* Prevent signed-wrap artifacts when objects move above/top or off left. */
    if (x < -7 || x > 255 || y < -7 || y > 239) {
        return;
    }

    base = (u8)(oam_used << 2);
    OAM[base + 0] = (u8)y;
    OAM[base + 1] = tile;
    OAM[base + 2] = attr;
    OAM[base + 3] = (u8)x;
    ++oam_used;
}

static void oam_put_visible(u8 x, u8 y, u8 tile, u8 attr) {
    u8 base;

    if (oam_used >= 64) {
        return;
    }
    base = (u8)(oam_used << 2);
    OAM[base + 0] = y;
    OAM[base + 1] = tile;
    OAM[base + 2] = attr;
    OAM[base + 3] = x;
    ++oam_used;
}

static void dma_oam(void) {
    OAMADDR = 0;
    OAMDMA = 0x02;
}

static void draw_character_icon(u8 character, u8 frame, s16 x, s16 y, u8 facing_right) {
    const unsigned int* frame_offsets;
    const u8* frame_counts;
    const u8* render_data;
    u8 pal = char_palette_id[character];
    u8 remaining;
    u16 frame_offset;

    y -= (ROSTER_FRAME_HEIGHT_TILES * 8 - FIGHTER_H);

    switch (character) {
        case CHAR_SIMON:
            frame_offsets = roster_render_offsets[CHAR_SIMON];
            frame_counts = roster_render_counts[CHAR_SIMON];
            break;
        case CHAR_TOMMY:
            frame_offsets = roster_render_offsets[CHAR_TOMMY];
            frame_counts = roster_render_counts[CHAR_TOMMY];
            break;
        case CHAR_RIZER:
            frame_offsets = roster_render_offsets[CHAR_RIZER];
            frame_counts = roster_render_counts[CHAR_RIZER];
            break;
        default:
            frame_offsets = roster_render_offsets[CHAR_MARIO];
            frame_counts = roster_render_counts[CHAR_MARIO];
            break;
    }
    frame_offset = frame_offsets[frame];
    remaining = frame_counts[frame];
    render_data = roster_render_bytes + frame_offset;

    if (x >= 16 && x <= 231 && y >= 0 && y <= 207) {
        u8 origin_x = (u8)x;
        u8 origin_y = (u8)y;

        while (remaining > 0 && oam_used < 64) {
            u8 base = (u8)(oam_used << 2);
            u8 x_offset = *render_data++;
            u8 y_offset = *render_data++;
            u8 tile = *render_data++;
            u8 attributes = (u8)(*render_data++ | pal);

            if (!facing_right) {
                attributes ^= 0x40;
            }
            OAM[base + 0] = (u8)(origin_y + y_offset);
            OAM[base + 1] = tile;
            OAM[base + 2] = attributes;
            OAM[base + 3] = facing_right
                ? (u8)(origin_x + x_offset)
                : (u8)(origin_x + 8 - x_offset);
            ++oam_used;
            --remaining;
        }
        return;
    }

    while (remaining > 0) {
        u8 x_offset = *render_data++;
        u8 y_offset = *render_data++;
        u8 tile = *render_data++;
        u8 attributes = (u8)(*render_data++ | pal);
        s16 tile_x;

        if (facing_right) {
            tile_x = (s16)(x + x_offset);
        } else {
            tile_x = (s16)(x + 8 - x_offset);
            attributes ^= 0x40;
        }
        oam_put(tile_x, (s16)(y + y_offset), tile, attributes);
        --remaining;
    }
}

static void spawn_hit_effect(const Fighter* target, s8 dir) {
    u8 i;
    HitEffect* effect = &hit_effects[0];

    for (i = 0; i < 2; ++i) {
        if (hit_effects[i].timer == 0) {
            effect = &hit_effects[i];
            break;
        }
    }
    effect->timer = 5;
    effect->palette = char_palette_id[target->character];
    effect->x = (s16)(target->x + (dir > 0 ? -4 : 12));
    effect->y = (s16)(target->y + 12);
}

static void apply_hit(Fighter* target, s8 dir, u8 damage, s8 vy) {
    u8 knock;

    if (target->damage > (u8)(255 - damage)) {
        target->damage = 255;
    } else {
        target->damage = (u8)(target->damage + damage);
    }
    knock = (u8)(4 + (target->damage >> 4));
    if (knock > 14) {
        knock = 14;
    }
    target->knock_vx = (s8)(dir < 0 ? -(s8)knock : (s8)knock);
    target->vy = vy;
    target->on_ground = 0;
    target->hitstun = 10;
    target->attack_timer = 0;
    target->special_timer = 0;
    target->attack_hit = 0;
    target->special_hit = 0;
    spawn_hit_effect(target, dir);
    music_sfx(MUSIC_SFX_HIT);
}

static u8 overlaps(s16 ax, s16 ay, u8 aw, u8 ah, s16 bx, s16 by, u8 bw, u8 bh) {
    if ((ax + aw) <= bx) return 0;
    if ((bx + bw) <= ax) return 0;
    if ((ay + ah) <= by) return 0;
    if ((by + bh) <= ay) return 0;
    return 1;
}

static u8 action_elapsed(u8 total, u8 remaining) {
    return remaining > total ? 0 : (u8)(total - remaining);
}

static u8 jab_phase(u8 character, u8 remaining) {
    u8 elapsed = action_elapsed(roster_jab_total_ticks[character], remaining);
    if (elapsed >= roster_jab_phase_2_start[character]) return 2;
    if (elapsed >= roster_jab_phase_1_start[character]) return 1;
    return 0;
}

static u8 special_phase(u8 character, u8 remaining) {
    u8 elapsed = action_elapsed(roster_special_total_ticks[character], remaining);
    if (elapsed >= roster_special_phase_2_start[character]) return 2;
    if (elapsed >= roster_special_phase_1_start[character]) return 1;
    return 0;
}

static u8 jab_is_active(u8 character, u8 remaining) {
    u8 elapsed = action_elapsed(roster_jab_total_ticks[character], remaining);
    return (u8)(
        remaining > 0 &&
        elapsed >= roster_jab_active_start[character] &&
        elapsed <= roster_jab_active_end[character]
    );
}

static u8 special_is_active(u8 character, u8 remaining) {
    u8 elapsed = action_elapsed(roster_special_total_ticks[character], remaining);
    return (u8)(
        remaining > 0 &&
        elapsed >= roster_special_active_start[character] &&
        elapsed <= roster_special_active_end[character]
    );
}

static Projectile* alloc_projectile(void) {
    u8 i;
    Projectile* p = projectiles;

    for (i = 0; i < MAX_PROJECTILES; ++i, ++p) {
        if (!p->active) {
            return p;
        }
    }
    return 0;
}

static void spawn_special_projectile(u8 owner) {
    Fighter* f = &fighters[owner];
    Projectile* p;
    s8 dir = (f->facing ? 1 : -1);
    u8 i;

    /* Keep at most one active projectile per fighter to bound frame cost. */
    p = projectiles;
    for (i = 0; i < MAX_PROJECTILES; ++i, ++p) {
        if (p->active && p->owner == owner) {
            return;
        }
    }

    p = alloc_projectile();
    if (!p) {
        return;
    }

    p->active = 1;
    p->owner = owner;
    p->palette = char_palette_id[f->character];
    p->x = (s16)(f->x + (dir > 0 ? 14 : -2));
    p->y = (s16)(f->y + 14);
    ++active_projectile_count;

    if (f->character == CHAR_MARIO) {
        p->kind = PROJECTILE_FIREBALL;
        p->vx = (s8)(dir * 3);
        p->vy = -1;
        p->ttl = 40;
    } else if (f->character == CHAR_SIMON) {
        p->kind = PROJECTILE_AXE;
        p->y = (s16)(f->y + 4);
        p->vx = (s8)(dir * 2);
        p->vy = -9;
        p->ttl = 55;
    } else {
        p->kind = PROJECTILE_BULLET;
        p->y = (s16)(f->y + 8);
        p->vx = (s8)(dir * 4);
        p->vy = 0;
        p->ttl = 30;
    }
}

static void reset_match(void) {
    u8 i;

    fighters[0].x = 56;
    fighters[0].y = GROUND_Y;
    fighters[0].vy = 0;
    fighters[0].knock_vx = 0;
    fighters[0].on_ground = 1;
    fighters[0].jumps_used = 0;
    fighters[0].facing = 1;
    fighters[0].attack_timer = 0;
    fighters[0].special_timer = 0;
    fighters[0].attack_hit = 0;
    fighters[0].special_hit = 0;
    fighters[0].hitstun = 0;

    fighters[1].x = 184;
    fighters[1].y = GROUND_Y;
    fighters[1].vy = 0;
    fighters[1].knock_vx = 0;
    fighters[1].on_ground = 1;
    fighters[1].jumps_used = 0;
    fighters[1].facing = 0;
    fighters[1].attack_timer = 0;
    fighters[1].special_timer = 0;
    fighters[1].attack_hit = 0;
    fighters[1].special_hit = 0;
    fighters[1].hitstun = 0;

    for (i = 0; i < MAX_PROJECTILES; ++i) {
        projectiles[i].active = 0;
    }
    active_projectile_count = 0;
    for (i = 0; i < 2; ++i) {
        hit_effects[i].timer = 0;
    }
}

static void enter_title(void) {
    u8 row_y;

    ppu_off();
    clear_nametable(TILE_BLANK);
    mode_cursor = 0;
    title_cursor_rendered = 0xFF;
    ppu_load_palette();
    draw_title_scene();
    draw_title_palette_zones();
    draw_title_menu_options();
    row_y = (u8)(TITLE_MENU_Y + (mode_cursor * TITLE_MENU_STEP));
    draw_tile(TITLE_CURSOR_TILE_X, row_y, TILE_ARROW_BG);
    title_cursor_rendered = mode_cursor;

    ppu_on();

    game_state = STATE_TITLE;
    music_set_scene(MUSIC_SCENE_TITLE);
}

static void enter_lobby(void) {
    ppu_off();
    ppu_load_palette();
    clear_nametable(TILE_BLANK);
    draw_lobby_layout();
    draw_lobby_palette_zones();

    ppu_on();

    game_state = STATE_LOBBY;
    music_set_scene(MUSIC_SCENE_LOBBY);
}

static void enter_battle(void) {
    u8 i;

    ppu_off();
    clear_nametable(TILE_BLANK);
    ppu_set_universal_background(0x21);
    draw_battle_layout();

    draw_text(5, 3, "P1:");
    draw_text(9, 3, character_names[p1_char]);
    draw_text(17, 3, "P2:");
    draw_text(21, 3, character_names[p2_char]);
    format_damage_text(0, battle_hud_text[0]);
    format_damage_text(0, battle_hud_text[1]);
    draw_battle_damage_text_bg(0, 0);
    draw_battle_damage_text_bg(1, 0);

    for (i = 0; i < PLATFORM_TILE_W; ++i) {
        draw_tile(
            (u8)(PLATFORM_TILE_X + i),
            PLATFORM_TILE_Y,
            (u8)((i & 1) ? TILE_PLATFORM_B : TILE_PLATFORM_A)
        );
        draw_tile(
            (u8)(PLATFORM_TILE_X + i),
            (u8)(PLATFORM_TILE_Y + 1),
            (u8)((i & 1) ? TILE_PLATFORM_A : TILE_PLATFORM_B)
        );
    }
    for (i = 0; i < 2; ++i) {
        draw_tile((u8)(PLATFORM_TILE_X + i), (u8)(PLATFORM_TILE_Y + 2), TILE_PLATFORM_A);
        draw_tile((u8)(PLATFORM_TILE_X + i), (u8)(PLATFORM_TILE_Y + 3), TILE_PLATFORM_B);
        draw_tile((u8)(PLATFORM_TILE_X + PLATFORM_TILE_W - 2 + i), (u8)(PLATFORM_TILE_Y + 2), TILE_PLATFORM_B);
        draw_tile((u8)(PLATFORM_TILE_X + PLATFORM_TILE_W - 2 + i), (u8)(PLATFORM_TILE_Y + 3), TILE_PLATFORM_A);
    }
    draw_battle_palette_zones();

    ppu_on();

    fighters[0].character = p1_char;
    fighters[1].character = p2_char;

    fighters[0].damage = 0;
    fighters[1].damage = 0;

    fighters[0].ko_count = 0;
    fighters[1].ko_count = 0;

    fighters[0].is_cpu = (game_mode == MODE_CPU_CPU) ? 1 : 0;
    fighters[1].is_cpu = (game_mode == MODE_2P) ? 0 : 1;

    fighters[0].control = 0;
    fighters[1].control = 0;
    fighters[0].prev_control = 0;
    fighters[1].prev_control = 0;

    battle_frame_counter = 0;
    battle_hud_prev_damage[0] = 0;
    battle_hud_prev_damage[1] = 0;
    battle_hud_dirty[0] = 0;
    battle_hud_dirty[1] = 0;
    reset_match();

    game_state = STATE_BATTLE;
    music_set_scene(MUSIC_SCENE_BATTLE);
}

static u8 ai_input_for(u8 idx) {
    Fighter* me = &fighters[idx];
    Fighter* foe = &fighters[idx ^ 1];
    u8 in = 0;
    s16 dx = (s16)(foe->x - me->x);
    u8 dist = absdiff8(foe->x, me->x);

    if (dx < -6) {
        in |= BTN_LEFT;
    } else if (dx > 6) {
        in |= BTN_RIGHT;
    }

    if (me->on_ground && foe->y + 8 < me->y && ((frame_counter & 0x0F) == 0)) {
        in |= BTN_UP;
    }

    if (!me->on_ground && me->jumps_used == 1 && me->y < (GROUND_Y - 24) && ((frame_counter & 0x0F) == 0)) {
        in |= BTN_UP;
    }

    if (dist < 18) {
        if ((frame_counter & 0x03) == 0) {
            in |= BTN_A;
        }
    } else {
        if ((frame_counter & 0x07) == 0) {
            in |= BTN_B;
        }
    }

    return in;
}

static void update_fighter_input(u8 idx, u8 input) {
    Fighter* f = &fighters[idx];

    f->control = input;

    if (f->hitstun > 0) {
        return;
    }

    if (input & BTN_LEFT) {
        f->x -= 2;
        f->facing = 0;
    }
    if (input & BTN_RIGHT) {
        f->x += 2;
        f->facing = 1;
    }

    if (rising_edge(input, f->prev_control, BTN_UP)) {
        if (f->jumps_used < 2) {
            f->vy = -9;
            f->on_ground = 0;
            ++f->jumps_used;
            music_sfx(MUSIC_SFX_JUMP);
        }
    }

    if (
        rising_edge(input, f->prev_control, BTN_A) &&
        f->attack_timer == 0 &&
        f->special_timer == 0
    ) {
        f->attack_timer = roster_jab_total_ticks[f->character];
        f->attack_hit = 0;
    }

    if (
        rising_edge(input, f->prev_control, BTN_B) &&
        f->special_timer == 0 &&
        f->attack_timer == 0
    ) {
        f->special_timer = roster_special_total_ticks[f->character];
        f->special_hit = 0;
        if (f->character != CHAR_TOMMY) {
            spawn_special_projectile(idx);
        }
    }
}

static void apply_fighter_physics(Fighter* f) {
    u8 on_platform_x;

    if (f->hitstun > 0) {
        --f->hitstun;
    }

    f->x += f->knock_vx;
    if (f->knock_vx > 0) {
        --f->knock_vx;
    } else if (f->knock_vx < 0) {
        ++f->knock_vx;
    }

    if (!f->on_ground || f->vy < 0) {
        if ((frame_counter & 0x01) == 0 && f->vy < 6) {
            ++f->vy;
        }
        f->y += f->vy;
    }

    /* Feet should contact platform top exactly at y=(PLATFORM_Y - FIGHTER_H). */
    on_platform_x = (u8)((f->x + FIGHTER_W) > PLATFORM_LEFT && f->x < PLATFORM_RIGHT);

    if (on_platform_x && f->y >= GROUND_Y) {
        f->y = GROUND_Y;
        f->vy = 0;
        f->on_ground = 1;
        f->jumps_used = 0;
    } else {
        f->on_ground = 0;
    }

    if (f->x < BLAST_LEFT || f->x > BLAST_RIGHT || f->y > BLAST_BOTTOM) {
        if (f == &fighters[0]) {
            fighters[1].ko_count = (u8)(fighters[1].ko_count + 1);
        } else {
            fighters[0].ko_count = (u8)(fighters[0].ko_count + 1);
        }
        f->damage = 0;
        f->x = (f == &fighters[0]) ? 56 : 184;
        f->y = GROUND_Y;
        f->vy = 0;
        f->knock_vx = 0;
        f->on_ground = 1;
        f->jumps_used = 0;
        f->hitstun = 0;
        f->attack_timer = 0;
        f->special_timer = 0;
        f->attack_hit = 0;
        f->special_hit = 0;
        music_sfx(MUSIC_SFX_KO);
    }
}

static void tick_action_timers(Fighter* f) {
    if (f->attack_timer > 0) {
        --f->attack_timer;
    }
    if (f->special_timer > 0) {
        --f->special_timer;
    }
}

static void update_hit_effects(void) {
    u8 i;
    for (i = 0; i < 2; ++i) {
        if (hit_effects[i].timer > 0) {
            --hit_effects[i].timer;
        }
    }
}

static void resolve_melee_hits(u8 attacker_idx) {
    Fighter* atk = &fighters[attacker_idx];
    Fighter* def = &fighters[attacker_idx ^ 1];
    s8 dir = atk->facing ? 1 : -1;
    s16 hx;
    s16 hy;
    u8 hw;
    u8 hh;

    /* A / jab */
    if (jab_is_active(atk->character, atk->attack_timer) && !atk->attack_hit) {
        hw = jab_range[atk->character];
        hh = 8;
        hy = (s16)(atk->y + 14);
        if (dir > 0) {
            hx = (s16)(atk->x + FIGHTER_W);
        } else {
            hx = (s16)(atk->x - hw);
        }

        if (overlaps(hx, hy, hw, hh, def->x, def->y, FIGHTER_W, FIGHTER_H)) {
            apply_hit(def, dir, jab_damage[atk->character], -6);
            atk->attack_hit = 1;
        }
    }

    /* B / Tommy long kick */
    if (
        atk->character == CHAR_TOMMY &&
        special_is_active(atk->character, atk->special_timer) &&
        !atk->special_hit
    ) {
        hw = 24;
        hh = 8;
        hy = (s16)(atk->y + 16);
        if (dir > 0) {
            hx = (s16)(atk->x + FIGHTER_W);
        } else {
            hx = (s16)(atk->x - hw);
        }

        if (overlaps(hx, hy, hw, hh, def->x, def->y, FIGHTER_W, FIGHTER_H)) {
            apply_hit(def, dir, 11, -8);
            atk->special_hit = 1;
        }
    }
}

static void update_projectiles(void) {
    u8 i;
    Projectile* p = projectiles;

    for (i = 0; i < MAX_PROJECTILES; ++i, ++p) {
        Fighter* target;
        s8 dir;

        if (!p->active) {
            continue;
        }

        p->x += p->vx;
        p->y += p->vy;

        if (p->kind == PROJECTILE_AXE) {
            if (p->vy < 6) {
                ++p->vy;
            }
        } else if (p->kind == PROJECTILE_FIREBALL) {
            if (p->vy < 4) {
                ++p->vy;
            }
            if (p->y >= (GROUND_Y + FIGHTER_H - 8)) {
                p->y = (GROUND_Y + FIGHTER_H - 8);
                p->vy = -3;
            }
        }

        if (p->ttl > 0) {
            --p->ttl;
        }

        if (p->ttl == 0 || p->x < BLAST_LEFT || p->x > BLAST_RIGHT || p->y > BLAST_BOTTOM) {
            p->active = 0;
            --active_projectile_count;
            continue;
        }

        target = &fighters[p->owner ^ 1];
        if (overlaps(p->x, p->y, 8, 8, target->x, target->y, FIGHTER_W, FIGHTER_H)) {
            dir = p->vx >= 0 ? 1 : -1;
            if (p->kind == PROJECTILE_BULLET) {
                apply_hit(target, dir, 10, -5);
            } else if (p->kind == PROJECTILE_AXE) {
                apply_hit(target, dir, 14, -8);
            } else {
                apply_hit(target, dir, 8, -5);
            }
            p->active = 0;
            --active_projectile_count;
        }
    }
}

static void draw_fighter(const Fighter* f) {
    u8 frame = roster_frame_idle_a[f->character];
    u8 phase;
    s16 x = f->x;
    s16 y = f->y;
    u8 move_lr = (u8)(f->control & (BTN_LEFT | BTN_RIGHT));

    if (f->hitstun > 0) {
        frame = roster_frame_hurt[f->character];
    } else if (!f->on_ground && roster_jump_looping[f->character]) {
        phase = (u8)(
            (battle_frame_counter / roster_jump_frame_ticks[f->character]) % 4
        );
        frame = roster_frame_jump[f->character][phase];
    } else if (f->special_timer > 0) {
        phase = special_phase(f->character, f->special_timer);
        frame = roster_frame_special[f->character][phase];
    } else if (f->attack_timer > 0) {
        phase = jab_phase(f->character, f->attack_timer);
        frame = roster_frame_jab[f->character][phase];
    } else if (!f->on_ground) {
        if (f->vy < -2) {
            phase = 0;
        } else if (f->vy <= 2) {
            phase = 1;
        } else {
            phase = 2;
        }
        frame = roster_frame_jump[f->character][phase];
    } else if (move_lr == BTN_LEFT || move_lr == BTN_RIGHT) {
        phase = (u8)((battle_frame_counter / roster_walk_frame_ticks[f->character]) % 4);
        frame = roster_frame_walk[f->character][phase];
    } else if (((battle_frame_counter / roster_idle_frame_ticks[f->character]) & 0x01) != 0) {
        frame = roster_frame_idle_b[f->character];
    }

    fighter_render_frame[f == &fighters[0] ? 0 : 1] = frame;
    draw_character_icon(f->character, frame, x, y, f->facing);
}

static void draw_projectiles(void) {
    u8 i;
    const Projectile* p = projectiles;

    for (i = 0; i < MAX_PROJECTILES; ++i, ++p) {
        u8 tile;
        u8 pal;
        u8 flip = 0;
        u8 phase;

        if (!p->active) {
            continue;
        }

        if (p->kind == PROJECTILE_AXE) {
            phase = (u8)((battle_frame_counter >> 2) & 0x03);
            tile = (u8)(SPR_AXE_FRAME_0 + (phase >> 1));
            if (phase & 0x01) {
                flip = 0x40;
            }
        } else if (p->kind == PROJECTILE_BULLET) {
            tile = SPR_BULLET_REAL;
        } else {
            phase = (u8)((battle_frame_counter >> 1) & 0x03);
            tile = (u8)(SPR_FIREBALL_FRAME_0 + (phase & 0x01));
            if (phase & 0x02) {
                flip = 0xC0;
            }
        }

        pal = p->palette;
        pal |= flip;
        if (p->x >= 0 && p->x <= 255 && p->y >= 0 && p->y <= 239) {
            oam_put_visible((u8)p->x, (u8)p->y, tile, pal);
        } else {
            oam_put(p->x, p->y, tile, pal);
        }
    }
}

static void draw_hit_effects(void) {
    u8 i;

    for (i = 0; i < 2; ++i) {
        const HitEffect* effect = &hit_effects[i];
        if (effect->timer > 0) {
            s16 x = (s16)(effect->x + ((effect->timer & 1) ? 1 : -1));
            if (x >= 0 && x <= 255 && effect->y >= 0 && effect->y <= 239) {
                oam_put_visible((u8)x, (u8)effect->y, SPR_HIT, effect->palette);
            } else {
                oam_put(x, effect->y, SPR_HIT, effect->palette);
            }
        }
    }
}

static void draw_simon_whip_extensions(void) {
    u8 i;
    s16 x0;
    s16 x1;
    s16 x2;

    for (i = 0; i < 2; ++i) {
        const Fighter* atk = &fighters[i];
        u8 pal;
        s16 y;

        if (atk->character != CHAR_SIMON) {
            continue;
        }

        if (!jab_is_active(atk->character, atk->attack_timer)) {
            continue;
        }

        pal = char_palette_id[CHAR_SIMON];
        y = (s16)(atk->y + 10);
        if (atk->facing) {
            x0 = (s16)(atk->x + 16);
            x1 = (s16)(atk->x + 24);
            x2 = (s16)(atk->x + 32);
        } else {
            pal = (u8)(pal | 0x40);
            x0 = (s16)(atk->x - 8);
            x1 = (s16)(atk->x - 16);
            x2 = (s16)(atk->x - 24);
        }
        if (x0 >= 0 && x0 <= 255 && x1 >= 0 && x1 <= 255 && x2 >= 0 && x2 <= 255 && y >= 0 && y <= 239) {
            oam_put_visible((u8)x0, (u8)y, atk->facing ? SPR_SIMON_WHIP_0 : SPR_SIMON_WHIP_MID, pal);
            oam_put_visible((u8)x1, (u8)y, SPR_SIMON_WHIP_MID, pal);
            oam_put_visible((u8)x2, (u8)y, atk->facing ? SPR_SIMON_WHIP_MID : SPR_SIMON_WHIP_0, pal);
        } else if (atk->facing) {
            oam_put(x0, y, SPR_SIMON_WHIP_0, pal);
            oam_put(x1, y, SPR_SIMON_WHIP_MID, pal);
            oam_put(x2, y, SPR_SIMON_WHIP_MID, pal);
        } else {
            oam_put(x0, y, SPR_SIMON_WHIP_MID, pal);
            oam_put(x1, y, SPR_SIMON_WHIP_MID, pal);
            oam_put(x2, y, SPR_SIMON_WHIP_0, pal);
        }
    }
}

static void draw_lobby_selection_marker(u8 character, u8 pal, u8 inset) {
    u8 x0 = (u8)(lobby_card_x[character] * 8 + inset);
    u8 y0 = (u8)(lobby_card_y[character] * 8 + inset);
    u8 x1 = (u8)(lobby_card_x[character] * 8 + (LOBBY_CELL_W * 8) - 8 - inset);
    u8 y1 = (u8)(lobby_card_y[character] * 8 + (LOBBY_CELL_H * 8) - 8 - inset);

    oam_put(x0, y0, SPR_CURSOR, pal);
    oam_put(x1, y0, SPR_CURSOR, (u8)(pal | 0x40));
    oam_put(x0, y1, SPR_CURSOR, (u8)(pal | 0x80));
    oam_put(x1, y1, SPR_CURSOR, (u8)(pal | 0xC0));
}

#if TITLE_VARIANT == TITLE_VARIANT_CHALLENGERS
static void draw_title_challengers(void) {
    static const u8 walk_cycle[4] = {1, 2, 3, 2};
    u8 phase = (u8)((frame_counter >> 3) & 0x03);

    draw_character_icon(CHAR_MARIO, walk_cycle[phase], 8, 168, 1);
    draw_character_icon(CHAR_SIMON, walk_cycle[(phase + 1) & 0x03], 56, 202, 1);
    draw_character_icon(CHAR_TOMMY, walk_cycle[(phase + 2) & 0x03], 184, 202, 0);
    draw_character_icon(CHAR_RIZER, walk_cycle[(phase + 3) & 0x03], 232, 168, 0);
}
#endif

static void draw_lobby_dynamic_sprites(void) {
    u8 i;

    for (i = 0; i < CHAR_COUNT; ++i) {
        u8 sx = (u8)(lobby_card_x[i] * 8 + ((LOBBY_CELL_W * 8 - FIGHTER_W) >> 1));
        u8 sy = (u8)(lobby_card_y[i] * 8 + ((LOBBY_CELL_H * 8 - FIGHTER_H) >> 1));
        draw_character_icon(i, 0, sx, sy, 1);
    }

    draw_lobby_selection_marker(p1_char, TITLE_PAL_1P, 0);
    if (p1_char == p2_char) {
        draw_lobby_selection_marker(p2_char, TITLE_PAL_2P, 8);
    } else {
        draw_lobby_selection_marker(p2_char, TITLE_PAL_2P, 0);
    }
}

static void update_title(void) {
    if (rising_edge(pad1, pad1_prev, BTN_UP) && mode_cursor > 0) {
        --mode_cursor;
        music_sfx(MUSIC_SFX_MENU);
    }
    if (rising_edge(pad1, pad1_prev, BTN_DOWN) && mode_cursor < 2) {
        ++mode_cursor;
        music_sfx(MUSIC_SFX_MENU);
    }

    if (rising_edge(pad1, pad1_prev, BTN_START) || rising_edge(pad1, pad1_prev, BTN_A)) {
        game_mode = mode_cursor;
        enter_lobby();
    }
}

static void update_lobby(void) {
    if (rising_edge(pad1, pad1_prev, BTN_LEFT)) {
        if (p1_char == 0) p1_char = CHAR_COUNT - 1;
        else --p1_char;
        music_sfx(MUSIC_SFX_MENU);
    }
    if (rising_edge(pad1, pad1_prev, BTN_RIGHT)) {
        p1_char = (u8)((p1_char + 1) % CHAR_COUNT);
        music_sfx(MUSIC_SFX_MENU);
    }

    if (game_mode == MODE_2P) {
        if (rising_edge(pad2, pad2_prev, BTN_LEFT)) {
            if (p2_char == 0) p2_char = CHAR_COUNT - 1;
            else --p2_char;
            music_sfx(MUSIC_SFX_MENU);
        }
        if (rising_edge(pad2, pad2_prev, BTN_RIGHT)) {
            p2_char = (u8)((p2_char + 1) % CHAR_COUNT);
            music_sfx(MUSIC_SFX_MENU);
        }
    } else {
        if (rising_edge(pad1, pad1_prev, BTN_UP)) {
            if (p2_char == 0) p2_char = CHAR_COUNT - 1;
            else --p2_char;
            music_sfx(MUSIC_SFX_MENU);
        }
        if (rising_edge(pad1, pad1_prev, BTN_DOWN)) {
            p2_char = (u8)((p2_char + 1) % CHAR_COUNT);
            music_sfx(MUSIC_SFX_MENU);
        }
    }

    if (rising_edge(pad1, pad1_prev, BTN_START) || rising_edge(pad1, pad1_prev, BTN_A)) {
        enter_battle();
    }

    if (rising_edge(pad1, pad1_prev, BTN_SELECT)) {
        enter_title();
    }
}

static void update_battle(void) {
    u8 i;

    ++battle_frame_counter;
    update_hit_effects();

    if ((pad1 & BTN_START) && (pad1 & BTN_SELECT)) {
        enter_title();
        return;
    }

    for (i = 0; i < 2; ++i) {
        u8 in;
        tick_action_timers(&fighters[i]);
        if (fighters[i].is_cpu) {
            in = ai_input_for(i);
        } else {
            in = (i == 0) ? pad1 : pad2;
        }
        update_fighter_input(i, in);
    }

    resolve_melee_hits(0);
    resolve_melee_hits(1);

    update_projectiles();

    apply_fighter_physics(&fighters[0]);
    apply_fighter_physics(&fighters[1]);

    if (battle_hud_prev_damage[0] != fighters[0].damage) {
        battle_hud_prev_damage[0] = fighters[0].damage;
        format_damage_text(fighters[0].damage, battle_hud_text[0]);
        battle_hud_dirty[0] = 1;
    }
    if (battle_hud_prev_damage[1] != fighters[1].damage) {
        battle_hud_prev_damage[1] = fighters[1].damage;
        format_damage_text(fighters[1].damage, battle_hud_text[1]);
        battle_hud_dirty[1] = 1;
    }

    fighters[0].prev_control = fighters[0].control;
    fighters[1].prev_control = fighters[1].control;
}

static void render_frame(void) {
    begin_oam();

    if (game_state == STATE_TITLE) {
#if TITLE_VARIANT == TITLE_VARIANT_CHALLENGERS
        draw_title_challengers();
#endif
        /* Title cursor is background tile based for hardware compatibility. */
    } else if (game_state == STATE_LOBBY) {
        draw_lobby_dynamic_sprites();
    } else if (game_state == STATE_BATTLE) {
        draw_hit_effects();
        draw_fighter(&fighters[0]);
        draw_fighter(&fighters[1]);
        draw_simon_whip_extensions();
        draw_projectiles();
    }
    finish_oam();
}

static void publish_test_state(void) {
    u8 presentation;

    TEST_RAM[0] = 0x53; /* 'S' */
    TEST_RAM[1] = 0x4E; /* 'N' */
    TEST_RAM[2] = game_state;
    TEST_RAM[3] = game_mode;
    TEST_RAM[4] = frame_counter;
    TEST_RAM[5] = pad1;
    TEST_RAM[6] = pad2;
    TEST_RAM[7] = active_projectile_count;

    TEST_RAM[8] = (u8)fighters[0].x;
    TEST_RAM[9] = (u8)((u16)fighters[0].x >> 8);
    TEST_RAM[10] = (u8)fighters[0].y;
    TEST_RAM[11] = (u8)((u16)fighters[0].y >> 8);
    TEST_RAM[12] = fighters[0].damage;
    TEST_RAM[13] = fighters[0].ko_count;

    TEST_RAM[14] = (u8)fighters[1].x;
    TEST_RAM[15] = (u8)((u16)fighters[1].x >> 8);
    TEST_RAM[16] = (u8)fighters[1].y;
    TEST_RAM[17] = (u8)((u16)fighters[1].y >> 8);
    TEST_RAM[18] = fighters[1].damage;
    TEST_RAM[19] = fighters[1].ko_count;
    TEST_RAM[20] = music_debug_scene;
    TEST_RAM[21] = (u8)music_debug_step;
    TEST_RAM[22] = (u8)(music_debug_step >> 8);
    TEST_RAM[23] = music_debug_last_sfx;
    TEST_RAM[24] = fighters[0].attack_timer;
    TEST_RAM[25] = fighters[0].special_timer;
    TEST_RAM[26] = (u8)fighters[0].hitstun;
    TEST_RAM[27] = fighters[1].attack_timer;
    TEST_RAM[28] = fighters[1].special_timer;
    TEST_RAM[29] = (u8)fighters[1].hitstun;
    TEST_RAM[30] = (u8)(fighter_render_frame[0] | (fighter_render_frame[1] << 4));
    presentation = oam_used > 63 ? 63 : oam_used;
    if (hit_effects[0].timer > 0) {
        presentation += 0x40;
    }
    if (hit_effects[1].timer > 0) {
        presentation += 0x40;
    }
    TEST_RAM[31] = presentation;
}

static void init_ppu(void) {
    /* Wait for PPU warm-up (recommended by NESdev). */
    while (!(PPUSTATUS & 0x80)) {
    }
    while (!(PPUSTATUS & 0x80)) {
    }

    ppu_off();
    ppu_load_palette();
    clear_nametable(TILE_BLANK);
    init_oam();
    dma_oam();
    ppu_on();
}

void main(void) {
    p1_char = CHAR_MARIO;
    p2_char = CHAR_SIMON;
    frame_counter = 0;

    init_ppu();
    music_init();
    enter_title();

    for (;;) {
        wait_vblank_strict();
        /* Prioritize OAM DMA at vblank start for hardware-stable sprite fetch. */
        dma_oam();
        refresh_title_cursor_bg();
        refresh_battle_damage_bg();

        pad1 = read_controller(0);
        pad2 = read_controller(1);

        if (game_state == STATE_TITLE) {
            update_title();
        } else if (game_state == STATE_LOBBY) {
            update_lobby();
        } else {
            update_battle();
        }

        music_update();
        render_frame();
        publish_test_state();

        pad1_prev = pad1;
        pad2_prev = pad2;
        ++frame_counter;
    }
}
