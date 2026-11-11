"""KidBright uAI Plus / MaixPy 4: draw on each captured frame, then show."""
from maix import app, camera, display, image


def main():
    screen = display.Display()
    cam = camera.Camera(640, 480, image.Format.FMT_RGB888)
    try:
        while not app.need_exit():
            frame = cam.read()
            frame.draw_string(20, 20, "Hello KidBright", scale=2,
                              color=image.Color.from_rgb(0, 255, 255))
            frame.draw_circle(frame.width() // 2, frame.height() // 2, 60,
                              color=image.Color.from_rgb(255, 0, 0), thickness=3)
            screen.show(frame)
    finally:
        cam.close()


if __name__ == "__main__":
    main()
