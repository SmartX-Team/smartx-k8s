<#import "template.ftl" as layout>
<@layout.registrationLayout displayMessage=!messagesPerField.existsError("username", "password") displayInfo=false; section>
  <#if section = "header">
    <div class="smartx-brand">
      <span class="smartx-brand__logo">
        <img src="${url.resourcesPath}/img/logo.svg" alt="LOGO">
      </span>
      <span class="smartx-brand__eyebrow">RESEARCH CLOUD</span>
      <span class="smartx-brand__title">Welcome back</span>
      <span class="smartx-brand__subtitle">Sign in with your account to continue.</span>
    </div>
  <#elseif section = "form">
    <div id="kc-form">
      <div id="kc-form-wrapper">
        <#if realm.password>
          <form id="kc-form-login" onsubmit="login.disabled = true; return true;" action="${url.loginAction}" method="post">
            <#if !usernameHidden??>
              <div class="${properties.kcFormGroupClass!}">
                <label for="username" class="${properties.kcLabelClass!}">ID</label>
                <div class="smartx-input">
                  <svg class="smartx-input__icon" viewBox="0 0 24 24" aria-hidden="true">
                    <path d="M12 12a4 4 0 1 0 0-8 4 4 0 0 0 0 8Zm7 8a7 7 0 0 0-14 0"/>
                  </svg>
                  <input
                    tabindex="1"
                    id="username"
                    class="${properties.kcInputClass!}"
                    name="username"
                    value="${(login.username!'')}"
                    type="text"
                    placeholder="Enter your ID"
                    autofocus
                    autocomplete="username"
                    aria-invalid="<#if messagesPerField.existsError('username','password')>true</#if>"
                  >
                </div>
                <#if messagesPerField.existsError("username", "password")>
                  <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                    ${kcSanitize(messagesPerField.getFirstError("username", "password"))?no_esc}
                  </span>
                </#if>
              </div>
            </#if>

            <div class="${properties.kcFormGroupClass!}">
              <label for="password" class="${properties.kcLabelClass!}">Password</label>
              <div class="smartx-input">
                <svg class="smartx-input__icon" viewBox="0 0 24 24" aria-hidden="true">
                  <rect x="5" y="10" width="14" height="10" rx="2"/>
                  <path d="M8 10V7a4 4 0 0 1 8 0v3"/>
                </svg>
                <input
                  tabindex="2"
                  id="password"
                  class="${properties.kcInputClass!}"
                  name="password"
                  type="password"
                  placeholder="Enter your password"
                  autocomplete="current-password"
                  aria-invalid="<#if messagesPerField.existsError('username','password')>true</#if>"
                >
              </div>
              <#if usernameHidden?? && messagesPerField.existsError("username", "password")>
                <span id="input-error" class="${properties.kcInputErrorMessageClass!}" aria-live="polite">
                  ${kcSanitize(messagesPerField.getFirstError("username", "password"))?no_esc}
                </span>
              </#if>
            </div>

            <div id="kc-form-buttons" class="${properties.kcFormGroupClass!}">
              <input
                type="hidden"
                id="id-hidden-input"
                name="credentialId"
                <#if auth.selectedCredential?has_content>value="${auth.selectedCredential}"</#if>
              >
              <button
                tabindex="3"
                class="${properties.kcButtonClass!} ${properties.kcButtonPrimaryClass!} ${properties.kcButtonBlockClass!} ${properties.kcButtonLargeClass!}"
                name="login"
                id="kc-login"
                type="submit"
              >
                <span>Sign in</span>
                <svg viewBox="0 0 24 24" aria-hidden="true">
                  <path d="m9 18 6-6-6-6"/>
                </svg>
              </button>
            </div>
          </form>
        </#if>
      </div>
    </div>
  </#if>
</@layout.registrationLayout>
