package ui

import (
	"xoon/utils"

	"github.com/rivo/tview"
)

// ShowLoginForm displays the login form for wallet unlock
func ShowLoginForm(app *tview.Application, rootFlex *tview.Flex, onSuccess func()) {
	var passwordFieldIndex int
	loginForm := tview.NewForm().
		AddTextView("Instructions", "Please input password for existing encrypted wallet to unlock unmineable solXEN Miner", 0, 2, false, false)

	passwordFieldIndex = loginForm.GetFormItemCount()
	loginForm.AddPasswordField("Password:", "", 32, '*', nil)

	loginForm.AddButton("Unlock", func() {
		password := loginForm.GetFormItem(passwordFieldIndex).(*tview.InputField).GetText()
		if utils.VerifyPassword(password) {
			onSuccess()
		} else {
			showErrorModal(app, rootFlex, "Invalid password")
		}
	}).
		AddButton("Quit", func() {
			app.Stop()
		})

	loginForm.SetBorder(true).SetTitle("Unlock umineable solXEN Miner")
	rootFlex.Clear()
	rootFlex.AddItem(loginForm, 0, 1, true)
}

// showErrorModal displays an error message in a modal dialog
func showErrorModal(app *tview.Application, rootFlex *tview.Flex, message string) {
	modal := tview.NewModal().
		SetText(message).
		AddButtons([]string{"OK"}).
		SetDoneFunc(func(buttonIndex int, buttonLabel string) {
			if buttonLabel == "OK" {
				app.SetRoot(rootFlex, true)
			}
		})

	app.SetRoot(modal, false)
}
