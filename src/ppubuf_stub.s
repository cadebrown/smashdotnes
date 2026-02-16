.export ppubuf_flush

.segment "CODE"

; cc65's NES crt0 calls ppubuf_flush() from NMI.
; We keep this as a no-op so game code can explicitly control OAM DMA timing.
.proc ppubuf_flush
    rts
.endproc
