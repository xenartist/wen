package ui

import (
	"fmt"
	"time"
	"xoon/utils"

	"github.com/gdamore/tcell/v2"
	"github.com/rivo/tview"
)

func CreateMainMenu() *tview.List {
	mainMenu := tview.NewList().
		AddItem(APP_NAME_STRING, "", 0, nil).
		AddItem(PASSWORD_MANAGER_STRING, "", 'a', nil).
		AddItem(SOLXEN_CPU_MINER_STRING, "", 'b', func() {
			UpdateCPUMinerPublicKeyTextView() // Update the Public Key text view
		}).
		AddItem(TOKEN_HARVEST_STRING, "", 'f', nil).
		AddItem(QUIT_STRING, "", 'q', nil).
		AddItem(AUTHOR_STRING, "", 0, nil)

	mainMenu.SetBorder(true).SetTitle("xoon")
	return mainMenu
}

func SetupMenuItemSelection(mainMenu *tview.List, switchView func(*tview.Flex, *tview.Flex, *tview.TextView), modules []ModuleUI) {

	moduleMap := make(map[string]ModuleUI)
	for i, module := range modules {
		moduleName := getModuleName(i)
		moduleMap[moduleName] = module
	}

	mainMenu.SetSelectedFunc(func(index int, mainText string, secondaryText string, shortcut rune) {
		if module, ok := moduleMap[mainText]; ok {
			switchView(module.DashboardFlex, module.ConfigFlex, module.LogView)
		}
	})
}

func getModuleName(index int) string {
	if index < len(ModuleNames) {
		return ModuleNames[index]
	}
	return fmt.Sprintf("Module %d", index+1)
}

func SetupInputCapture(app *tview.Application, funcToRun func()) {
	var quitCount int
	var lastQuitTime time.Time

	app.SetInputCapture(func(event *tcell.EventKey) *tcell.EventKey {
		if event.Rune() == 'q' {
			now := time.Now()
			if now.Sub(lastQuitTime) > time.Second {
				quitCount = 1
			} else {
				quitCount++
			}
			lastQuitTime = now
			if quitCount >= 4 {
				utils.ClearGlobalKeys()
				app.Stop()
				return nil
			}
		}
		return event
	})
}
