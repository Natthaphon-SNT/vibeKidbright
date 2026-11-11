"""KidBright uAI Plus / MaixPy 4: show camera frames on the display."""
from maix import app, camera, display, image


def main():
    screen = display.Display()
    cam = camera.Camera(640, 480, image.Format.FMT_RGB888)
    try:
        while not app.need_exit():
            frame = cam.read()
            screen.show(frame)
    finally:
        cam.close()


if __name__ == "__main__":
    main()
