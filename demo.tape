# This is a vhs script. See https://github.com/charmbracelet/vhs for more info.
# To run this script, install vhs and run `vhs demo.tape`
Output "target/demo.gif"
Set Theme "Aardvark Blue"
Set FontSize 12
Set Width 1200
Set Height 800

#Hide
#Show
Type "cargo run"
Enter
Sleep 1s

########################################
# Navigate down to the Docker template #
########################################
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s

Type "l"
Sleep 0.5s

########################################
# Navigate down to the Dockerfile      #
########################################
Type "j"
Sleep 0.5s
Type "j"
Sleep 0.5s

########################################
# View the Dockerfile then quit        #
########################################
Type "l"
Sleep 2s
Type "q"
Sleep 0.5s

Type " "
Sleep 0.5s

########################################
# Navigate down to the Makefile        #
########################################
Type "j"
Sleep 0.5s

Type " "
Sleep 0.5s

########################################
# Extract the selected files           #
########################################

Type "e"
Sleep 9s

Type "git status"
Enter
Sleep 9s
