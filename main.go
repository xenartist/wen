package main

import (
	"xoon/ui"
	"xoon/utils"

	"github.com/rivo/tview"
)

var rootFlex *tview.Flex
var mainFlex *tview.Flex
var app *tview.Application

func main() {
	utils.PasswordProtectionInit()
	utils.XoosInit()

	app = tview.NewApplication()
	rootFlex = tview.NewFlex().SetDirection(tview.FlexRow)

	// Check for existing wallet
	partialPublicKey := utils.CheckExistingWallet()

	if partialPublicKey != "" {
		// Wallet exists, show login screen
		ui.ShowLoginForm(app, rootFlex, func() {
			showMainInterface(app)
		})
	} else {
		// No wallet exists, show main interface directly
		showMainInterface(app)
	}

	ui.SetupInputCapture(app, func() {
		// Clean up all UI elements
		rootFlex.Clear()
		mainFlex = nil
	})

	app.SetRoot(rootFlex, true).EnableMouse(true)
	if err := app.Run(); err != nil {
		utils.ClearGlobalKeys()
		panic(err)
	}
}

func showMainInterface(app *tview.Application) {
	mainMenu := ui.CreateMainMenu()
	rightFlex := tview.NewFlex().SetDirection(tview.FlexRow)

	walletUI := ui.CreateWalletUI(app)
	solXENCPUUI := ui.CreateSolXENCPUUI(app)
	switchView := ui.CreateSwitchViewFunc(rightFlex, mainMenu)

	modules := []ui.ModuleUI{
		{
			DashboardFlex: walletUI.DashboardFlex,
			ConfigFlex:    walletUI.ConfigFlex,
			LogView:       walletUI.LogView,
		},
		{
			DashboardFlex: solXENCPUUI.DashboardFlex,
			ConfigFlex:    solXENCPUUI.ConfigFlex,
			LogView:       solXENCPUUI.LogView,
		},
	}

	ui.SetupMenuItemSelection(mainMenu, switchView, modules)

	mainFlex = tview.NewFlex().
		AddItem(mainMenu, 0, 1, true).
		AddItem(rightFlex, 0, 3, false)

	rootFlex.Clear()
	rootFlex.AddItem(mainFlex, 0, 1, true)
}
