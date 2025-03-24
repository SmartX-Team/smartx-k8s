use yew::prelude::*;

#[function_component(TemplatePointers)]
fn component() -> Html {
    html! {
        <>
        // FIXME: https://tailwind-dashboard-template-dashwind.vercel.app/login
            <h1 class="text-2xl mt-8 font-bold">{ "Admin Dashboard Starter Kit" }</h1>
            // <p class="py-2 mt-4">✓ <span class="font-semibold">Light/dark</span> mode toggle</p>
            // <p class="py-2">✓ <span class="font-semibold">Redux toolkit</span> and other utility libraries configured</p>
            // <p class="py-2">✓ <span class="font-semibold">Calendar, Modal, Sidebar </span> components</p>
            // <p class="py-2">✓ User-friendly <span class="font-semibold">documentation</span></p>
            // <p class="py-2 mb-4">✓ <span class="font-semibold">Daisy UI</span> components, <span class="font-semibold">Tailwind CSS</span> support</p>
        </>
    }
}

#[function_component(LandingIntro)]
fn component() -> Html {
    html! {
        <div class="hero min-h-full rounded-l-xl bg-base-200">
            <div class="hero-content py-12">
                <div class="max-w-md">
                    // Header
                    <h1 class="text-3xl text-center font-bold ">
                        <img src="/public/logo192.png" class="w-12 inline-block mr-2 mask mask-circle" alt="dashwind-logo" />
                        { "MobileX" }
                    </h1>
                    // Body image
                    <div class="text-center mt-12">
                        <img src="/public/intro.png" alt="Dashwind Admin Template" class="w-48 inline-block" />
                    </div>
                    // Importing pointers component
                    <TemplatePointers />
                </div>
            </div>
        </div>
    }
}

#[function_component(Login)]
pub fn component() -> Html {
    let loading = use_state(|| false);

    html! {
        <div class="min-h-screen bg-base-200 flex items-center">
            <div class="card mx-auto w-full max-w-5xl shadow-xl">
                <div class="grid md:grid-cols-2 grid-cols-1 bg-base-100 rounded-xl">
                    <div>
                        <LandingIntro />
                    </div>
                    <div class="py-24 px-10">
                        <h2 class="text-2xl font-semibold mb-2 text-center">{ "Login" }</h2>
                        <form /* onSubmit={ submitForm } */ >
                            <div class="mb-4">
                                <div>
                                    <label class="input validator">
                                        <svg class="h-[1em] opacity-50" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
                                            <g stroke-linejoin="round" stroke-linecap="round" stroke-width="2.5" fill="none" stroke="currentColor">
                                                <rect width="20" height="16" x="2" y="4" rx="2" />
                                                <path d="m22 7-8.97 5.7a1.94 1.94 0 0 1-2.06 0L2 7" />
                                            </g>
                                        </svg>
                                        <input type="email" required=true
                                            placeholder="Email ID"
                                            /* updateFormValue={ updateFormValue } */
                                        />
                                    </label>
                                    <div class="validator-hint hidden">
                                        { "Enter valid email address" }
                                    </div>
                                </div>
                                <div>
                                    <label class="input validator">
                                        <svg class="h-[1em] opacity-50" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24">
                                            <g stroke-linejoin="round" stroke-linecap="round" stroke-width="2.5" fill="none" stroke="currentColor">
                                                <path d="M2.586 17.414A2 2 0 0 0 2 18.828V21a1 1 0 0 0 1 1h3a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h1a1 1 0 0 0 1-1v-1a1 1 0 0 1 1-1h.172a2 2 0 0 0 1.414-.586l.814-.814a6.5 6.5 0 1 0-4-4z" />
                                                <circle cx="16.5" cy="7.5" r=".5" fill="currentColor" />
                                            </g>
                                        </svg>
                                        <input type="password" required=true
                                            placeholder="Password"
                                            minlength="8"
                                            pattern=r"(?=.*\d)(?=.*[a-z])(?=.*[A-Z]).{8,}"
                                            title="Must be more than 8 characters, including number, lowercase letter, uppercase letter"
                                            /* updateFormValue={ updateFormValue } */
                                        />
                                    </label>
                                    <p class="validator-hint hidden">
                                        { "Must be more than 8 characters, including" }
                                        <br/>{ "At least one number" }
                                        <br/>{ "At least one lowercase letter" }
                                        <br/>{ "At least one uppercase letter" }
                                    </p>
                                </div>
                            </div>

                            // <div class="text-right text-primary">
                            //     <Link to="/forgot-password">
                            //         <span class="text-sm inline-block hover:text-primary hover:underline hover:cursor-pointer transition duration-200">{ "Forgot Password?" }</span>
                            //     </Link>
                            // </div>

                            // <ErrorText style="mt-8">{errorMessage}</ErrorText>
                            <button type="submit" class={
                                format!("btn mt-2 w-full btn-primary{}", if *loading {
                                    " loading"
                                } else {
                                    ""
                                })
                            }>{
                                "Login"
                            }</button>

                            // <div class="text-center mt-4">{ "Don't have an account yet?" }
                            //     <Link to="/register">
                            //         <span class="inline-block hover:text-primary hover:underline hover:cursor-pointer transition duration-200">{ "Register" }</span>
                            //     </Link>
                            // </div>
                        </form>
                    </div>
                </div>
            </div>
        </div>
    }
}
