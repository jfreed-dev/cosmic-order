# docs/screensaver.tape — regenerate with: vhs docs/screensaver.tape
#
# Renders the COSMIC ORDER screensaver's TerminalTextEffects "beams" effect on
# the brand logo into docs/screensaver.gif, in a headless terminal (no capture
# of the live desktop). Mirrors the niri-screensaver demo style.
Output docs/screensaver.gif

Set FontSize 16
Set Width 820
Set Height 800
Set Padding 24
Set Theme { "name": "cosmic", "background": "#1e1e2e", "foreground": "#cdd6f4", "cursor": "#f5e0dc", "black": "#45475a", "red": "#f38ba8", "green": "#a6e3a1", "yellow": "#f9e2af", "blue": "#89b4fa", "magenta": "#cba6f7", "cyan": "#94e2d5", "white": "#bac2de", "brightBlack": "#585b70", "brightRed": "#f38ba8", "brightGreen": "#a6e3a1", "brightYellow": "#f9e2af", "brightBlue": "#89b4fa", "brightMagenta": "#cba6f7", "brightCyan": "#94e2d5", "brightWhite": "#a6adc8" }

Hide
Type "clear && cat resources/screensaver/logos/cosmic-name-with-icon.txt | tte beams --final-gradient-stops 89b4fa cba6f7 --final-gradient-direction diagonal --beam-gradient-stops 89b4fa cba6f7 ; tput civis ; sleep 10"
Enter
Sleep 500ms
Show
Sleep 8s
