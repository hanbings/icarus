import {createBrowserRouter, RouterProvider} from "react-router-dom";
import {LoginScreen} from "./screens/Login.tsx";
import {Provider} from "react-redux";
import {configureStore} from "@reduxjs/toolkit";
import {useTokenStore} from "./stores/token.ts";
import {HomeScreen} from "./screens/Home.tsx";
import {ErrorScreen} from "./screens/Error.tsx";
import {NotFoundScreen} from "./screens/NotFound.tsx";
import {QueryClient, QueryClientProvider} from "@tanstack/react-query";

export default function App() {
    const store = configureStore({
        reducer: {
            token: useTokenStore.reducer
        },
    });

    const router = createBrowserRouter([
        {path: "/", element: <LoginScreen/>},
        {path: "/login", element: <LoginScreen/>},
        {path: "/home", element: <HomeScreen/>},
        {path: "/error", element: <ErrorScreen/>},
        {path: "*", element: <NotFoundScreen/>},
    ])

    const query = new QueryClient()

    return (
        <QueryClientProvider client={query}>
            <Provider store={store}>
                <RouterProvider router={router}/>
            </Provider>
        </QueryClientProvider>
    )
}