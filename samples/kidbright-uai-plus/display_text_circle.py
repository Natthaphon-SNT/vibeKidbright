"""KidBright uAI Plus / MaixPy 4: image buffer -> draw -> display.

Based on the supplied English training slides, page 16, and MaixPy APIs.
Run on the board, not in desktop Python. Physical output needs board testing.
"""
from maix import app, display, image, time


def main():
    screen = display.Display()
    width, height = screen.width(), screen.height()
    canvas = image.Image(width, height, image.Format.FMT_RGB888)
    canvas.draw_rect(0, 0, width, height,
                     color=image.Color.from_rgb(0, 0, 0), thickness=-1)
    canvas.draw_string(20, 20, "Hello KidBright", scale=2,
                       color=image.Color.from_rgb(0, 255, 255))
    canvas.draw_circle(width // 2, height // 2, min(width, height) // 6,
                       color=image.Color.from_rgb(255, 0, 0), thickness=3)
    while not app.need_exit():
        screen.show(canvas)
        time.sleep_ms(50)


if __name__ == "__main__":
    main()
