import { run, StateHolder } from "renderer-web";

const canvas: HTMLCanvasElement = document.getElementById("renderer-canvas") as HTMLCanvasElement;

let currentStateHolder: StateHolder | undefined;

// Resize
window.addEventListener("resize", evt => resizeCanvas());
function resizeCanvas() {
    const width = document.body.clientWidth;
    const height = document.body.clientHeight;
    canvas.width = width;
    canvas.height = height;
    currentStateHolder?.resize(width, height);
}
resizeCanvas();

// Fov
window.addEventListener("wheel", evt => {
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
document.addEventListener("mousemove", (evt) => {
    if (document.pointerLockElement == canvas) {
        const movementX = evt.movementX || 0;
        const movementY = evt.movementY || 0;
        currentStateHolder?.update_rotation(movementX, movementY);
    }
});

// Initialize
run(() => {
    requestAnimationFrame(() => {
        currentStateHolder?.render();
    });
}, (stateHolder: StateHolder) => {
    currentStateHolder = stateHolder;
    requestAnimationFrame(() => {
        stateHolder.render();
    });
});
