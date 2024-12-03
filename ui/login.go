package ui

import (
	"fmt"
	"xoon/utils"

	"github.com/rivo/tview"
)

var (
	tempQuestions []string
	tempAnswers   []string
)

// ShowLoginForm displays the login form for wallet unlock
func ShowLoginForm(app *tview.Application, rootFlex *tview.Flex, onSuccess func()) {
	var passwordFieldIndex int
	loginForm := tview.NewForm().
		AddTextView("Instruction", "Please input password to unlock wen app", 0, 2, false, false)

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

	loginForm.SetBorder(true).SetTitle("Unlock wen App - X1/Solana Validator Wallet")
	rootFlex.Clear()
	rootFlex.AddItem(nil, 0, 1, false)
	rootFlex.AddItem(loginForm, 0, 2, true)
	rootFlex.AddItem(nil, 0, 1, false)
}

func ShowCreatePasswordForm(app *tview.Application, rootFlex *tview.Flex, onSuccess func()) {
	var password1FieldIndex, password2FieldIndex int
	createForm := tview.NewForm().
		AddTextView("Instructions", "Create a new password for your wallet", 0, 2, false, false)

	password1FieldIndex = createForm.GetFormItemCount()
	createForm.AddPasswordField("New Password:", "", 32, '*', nil)

	password2FieldIndex = createForm.GetFormItemCount()
	createForm.AddPasswordField("Confirm Password:", "", 32, '*', nil)

	createForm.AddButton("Create", func() {
		password1 := createForm.GetFormItem(password1FieldIndex).(*tview.InputField).GetText()
		password2 := createForm.GetFormItem(password2FieldIndex).(*tview.InputField).GetText()

		if password1 == "" {
			showErrorModal(app, rootFlex, "Password cannot be empty")
			return
		}

		if password1 != password2 {
			showErrorModal(app, rootFlex, "Passwords do not match")
			return
		}

		// Save the new password
		if err := utils.SaveNewPassword(password1); err != nil {
			showErrorModal(app, rootFlex, "Failed to save password: "+err.Error())
			return
		}

		// Show recovery questions setup form after password is created
		ShowRecoverySetupForm(app, rootFlex, onSuccess)
	}).
		AddButton("Quit", func() {
			app.Stop()
		})

	createForm.SetBorder(true).SetTitle("wen - X1/Solana Validator Wallet")
	rootFlex.Clear()
	// Center the form
	rootFlex.AddItem(nil, 0, 1, false)
	rootFlex.AddItem(createForm, 0, 2, true)
	rootFlex.AddItem(nil, 0, 1, false)
}

// ShowRecoverySetupForm displays the form for setting up recovery questions
func ShowRecoverySetupForm(app *tview.Application, rootFlex *tview.Flex, onSuccess func()) {
	// Clear any existing temporary data
	tempQuestions = nil
	tempAnswers = nil

	recoveryForm := tview.NewForm().
		AddTextView("Instructions", "Set up 3 security questions for password recovery", 0, 2, false, false)

	questionFields := make([]int, 3)
	answerFields := make([]int, 3)

	for i := 0; i < 3; i++ {
		questionFields[i] = recoveryForm.GetFormItemCount()
		recoveryForm.AddInputField(fmt.Sprintf("Question %d:", i+1), "", 50, nil, nil)

		answerFields[i] = recoveryForm.GetFormItemCount()
		recoveryForm.AddInputField(fmt.Sprintf("Answer %d:", i+1), "", 50, nil, nil)
	}

	recoveryForm.AddButton("Save", func() {
		// Collect all questions and answers
		questions := make([]string, 3)
		answers := make([]string, 3)

		for i := 0; i < 3; i++ {
			questions[i] = recoveryForm.GetFormItem(questionFields[i]).(*tview.InputField).GetText()
			answers[i] = recoveryForm.GetFormItem(answerFields[i]).(*tview.InputField).GetText()

			if questions[i] == "" || answers[i] == "" {
				showErrorModal(app, rootFlex, "All questions and answers must be filled")
				return
			}
		}

		// Store in temporary variables
		tempQuestions = questions
		tempAnswers = answers

		// Show verification form
		ShowRecoveryVerificationForm(app, rootFlex, onSuccess)
	}).
		AddButton("Quit", func() {
			app.Stop()
		})

	recoveryForm.SetBorder(true).SetTitle("Set Recovery Questions")
	rootFlex.Clear()
	rootFlex.AddItem(nil, 0, 1, false)
	rootFlex.AddItem(recoveryForm, 0, 2, true)
	rootFlex.AddItem(nil, 0, 1, false)
}

// ShowRecoveryVerificationForm displays the form for verifying recovery answers
func ShowRecoveryVerificationForm(app *tview.Application, rootFlex *tview.Flex, onSuccess func()) {
	verificationForm := tview.NewForm().
		AddTextView("Instructions", "Please verify your recovery answers", 0, 2, false, false)

	answerFields := make([]int, 3)

	// Display questions and add answer fields
	for i := 0; i < 3; i++ {
		verificationForm.AddTextView(fmt.Sprintf("Question %d:", i+1), tempQuestions[i], 0, 1, false, false)
		answerFields[i] = verificationForm.GetFormItemCount()
		verificationForm.AddInputField("Your Answer:", "", 50, nil, nil)
	}

	verificationForm.AddButton("Verify", func() {
		// Check all answers
		for i := 0; i < 3; i++ {
			userAnswer := verificationForm.GetFormItem(answerFields[i]).(*tview.InputField).GetText()
			if userAnswer != tempAnswers[i] {
				showErrorModal(app, rootFlex, fmt.Sprintf("Answer %d is incorrect. Please try again.", i+1))
				return
			}
		}

		// All answers are correct, save to storage
		if err := utils.SaveRecoveryQuestions(tempQuestions, tempAnswers); err != nil {
			showErrorModal(app, rootFlex, "Failed to save recovery questions: "+err.Error())
			return
		}

		// Clear temporary data
		tempQuestions = nil
		tempAnswers = nil

		// Show success form instead of modal
		successForm := tview.NewForm().
			AddTextView("Success", "Recovery questions verified and saved successfully!", 0, 2, false, false).
			AddButton("Continue", func() {
				onSuccess()
			})

		successForm.SetBorder(true).SetTitle("Success")
		rootFlex.Clear()
		rootFlex.AddItem(nil, 0, 1, false)
		rootFlex.AddItem(successForm, 0, 2, true)
		rootFlex.AddItem(nil, 0, 1, false)
	}).
		AddButton("Back", func() {
			ShowRecoverySetupForm(app, rootFlex, onSuccess)
		}).
		AddButton("Quit", func() {
			app.Stop()
		})

	verificationForm.SetBorder(true).SetTitle("Verify Recovery Questions")
	rootFlex.Clear()
	rootFlex.AddItem(nil, 0, 1, false)
	rootFlex.AddItem(verificationForm, 0, 2, true)
	rootFlex.AddItem(nil, 0, 1, false)
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
