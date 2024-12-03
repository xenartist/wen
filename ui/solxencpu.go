package ui

import (
	"xoon/utils"

	"github.com/rivo/tview"
)

var solxencpuForm *tview.Form = tview.NewForm()

func CreateSolXENCPUUI(app *tview.Application) ModuleUI {
	var moduleUI = CreateModuleUI(SOLXEN_CPU_MINER_STRING, app)

	// Determine the public key display text
	publicKeyDisplay := ""
	if utils.GetGlobalPublicKey() != "" {
		publicKeyDisplay = utils.GetGlobalPublicKey()[:8] + "********"
	}

	solxencpuForm.AddTextView("Public Key", publicKeyDisplay, 0, 1, false, true)

	contentFlex := tview.NewFlex().AddItem(solxencpuForm, 0, 1, true)

	moduleUI.ConfigFlex.AddItem(contentFlex, 0, 1, true)

	return moduleUI
}

func CreateSolXENCPUConfigFlex(app *tview.Application, logView *tview.TextView) *tview.Flex {
	configFlex := tview.NewFlex().
		SetDirection(tview.FlexColumn)

	configFlex.SetBorder(true).SetTitle(SOLXEN_CPU_MINER_STRING + " Config")
	return configFlex
}

func UpdateCPUMinerPublicKeyTextView() {
	if solxencpuForm == nil {
		return
	}

	if utils.GetGlobalPublicKey() == "" {
		solxencpuForm.GetFormItem(0).(*tview.TextView).SetText("")
	} else {
		solxencpuForm.GetFormItem(0).(*tview.TextView).SetText(utils.GetGlobalPublicKey()[:8] + "********")
	}
}
