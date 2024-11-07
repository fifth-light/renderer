import { MouseButton, run, StateHolder } from "renderer-web";

const canvas: HTMLCanvasElement = document.getElementById("renderer-canvas") as HTMLCanvasElement;

let currentStateHolder: StateHolder | undefined;

function posConvert(evt: MouseEvent): [number, number] {
    const posX = (evt.offsetX || 0) / canvas.clientWidth * canvas.width;
    const posY = (evt.offsetY || 0) / canvas.clientHeight * canvas.height;
    return [posX, posY];
}

// Fov
canvas.addEventListener("wheel", evt => {
    if (evt.deltaY > 0) {
        currentStateHolder?.update_fov(true);
    } else if (evt.deltaY < 0) {
        currentStateHolder?.update_fov(false);
    }
});

// Position
let guiActive = false;
window.addEventListener("keydown", evt => {
    switch (evt.key) {
        case "w":
        case "W":
            currentStateHolder?.set_forward(1.0);
            break;
        case "s":
        case "S":
            currentStateHolder?.set_backward(1.0);
            break;
        case "a":
        case "A":
            currentStateHolder?.set_left(1.0);
            break;
        case "d":
        case "D":
            currentStateHolder?.set_right(1.0);
            break;
        case "Shift":
            currentStateHolder?.set_down(1.0);
            break;
        case " ":
            currentStateHolder?.set_up(1.0);
            break;
        case "F10":
            let currentActive = currentStateHolder?.egui_active() || false;
            currentStateHolder?.set_egui_active(!currentActive);
            currentActive = currentStateHolder?.egui_active() || false
            guiActive = currentActive;

            if (guiActive) {
                document.exitPointerLock();
            } else {
                canvas.requestPointerLock();
            }
            break;
        default:
            return;
    }
    evt.preventDefault();
});
window.addEventListener("keyup", evt => {
    switch (evt.key) {
        case "w":
        case "W":
            currentStateHolder?.set_forward(0.0);
            break;
        case "s":
        case "S":
            currentStateHolder?.set_backward(0.0);
            break;
        case "a":
        case "A":
            currentStateHolder?.set_left(0.0);
            break;
        case "d":
        case "D":
            currentStateHolder?.set_right(0.0);
            break;
        case "Shift":
            currentStateHolder?.set_down(0.0);
            break;
        case " ":
            currentStateHolder?.set_up(0.0);
            break;
        default:
            return;
    }
    evt.preventDefault();
});

// Rotation
canvas.addEventListener("click", () => {
    if (!guiActive) {
        canvas.requestPointerLock();
    }
});
canvas.addEventListener("mousemove", (evt) => {
    if (document.pointerLockElement == canvas) {
        const deltaX = evt.movementX || 0;
        const deltaY = evt.movementY || 0;
        currentStateHolder?.update_rotation(deltaX, deltaY);
    }
    const [posX, posY] = posConvert(evt);
    currentStateHolder?.mouse_moved(posX, posY);
});

// Theme
const darkModeMediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
const lightModeMediaQuery = window.matchMedia("(prefers-color-scheme: light)");
function updateTheme() {
    if (darkModeMediaQuery.matches) {
        currentStateHolder?.set_theme(true);
    } else if (lightModeMediaQuery.matches) {
        currentStateHolder?.set_theme(false);
    } else {
        currentStateHolder?.set_theme(undefined);
    }
}
darkModeMediaQuery.addEventListener("change", updateTheme);
lightModeMediaQuery.addEventListener("change", updateTheme);
updateTheme();

// Focus
window.addEventListener("focus", () => {
    currentStateHolder?.set_focused(true);
})
window.addEventListener("blur", () => {
    currentStateHolder?.set_focused(false);
})

// Pointer
function mapMouseButton(button: number) {
    switch (button) {
        case 0:
            return MouseButton.Primary;
        case 1:
            return MouseButton.Middle;
        case 2:
            return MouseButton.Secondary;
        case 3:
            return MouseButton.Extra1;
        case 4:
            return MouseButton.Extra2;
        default:
            return null;
    };
}
canvas.addEventListener("mousedown", (evt) => {
    const [posX, posY] = posConvert(evt);
    const button = mapMouseButton(evt.button);
    if (button == null) {
        return;
    }
    currentStateHolder?.mouse_button(posX, posY, button, true);
});
canvas.addEventListener("mouseup", (evt) => {
    const [posX, posY] = posConvert(evt);
    const button = mapMouseButton(evt.button);
    if (button == null) {
        return;
    }
    currentStateHolder?.mouse_button(posX, posY, button, false);
});

// Initialize
let stateCreating = false;
const observer = new ResizeObserver((entries) => {
    let entry = entries.find(entry => entry.target == canvas)!!;
    let contentBoxSize = entry.contentBoxSize[0];
    const width = canvas.width = contentBoxSize.inlineSize;
    const height = canvas.height = contentBoxSize.blockSize;

    if (currentStateHolder == null) {
        if (stateCreating) {
            return;
        }
        stateCreating = true;
        run(() => {
            requestAnimationFrame(() => {
                currentStateHolder?.render();
            });
        }, (stateHolder: StateHolder) => {
            currentStateHolder = stateHolder;
            stateCreating = false;
            requestAnimationFrame(() => {
                stateHolder.render();
            });
        });
    } else {
        currentStateHolder?.resize(width, height, 1);
    }
});
observer.observe(canvas, { box: "device-pixel-content-box" });

