#include <Wire.h>
#include <MPU6050.h>
#include <PID_v1.h>
#include <esp_now.h>
#include <WiFi.h>
#include <PubSubClient.h>

const char* ssid = "Infinix SMART 9";
const char* password = "1357924680";
const char* mqtt_server = "broker.hivemq.com";

WiFiClient espClient;
PubSubClient client(espClient);
// ===== MPU =====
MPU6050 mpu;

// ===== ENCODER =====
#define ENCA 32
#define ENCB 33
volatile long pos = 0;
long prevPos = 0;
const int PPR = 330;

// ===== MOTOR =====
int LT = 18, LB = 19, RT = 26, RB = 27;

// ===== VARIABLES =====
double Angle, PWM, Delta_ang;
double Setpoint = 0;
double Setpo = 0;
double currPos;
double currPose;

// PID (Angle)  
double Kp = 16;
double Ki = 0;
double Kd = 0.5;

double Kppo = 2.12;
double Kipo = 0;
double Kdpo = 1.4;


// Kc
double Kc = 0.7;

// RPM
float rpm = 0;

typedef struct struct_message {
    int x, y;
    bool EN_lock =1;
} struct_message;

// Create a struct_message called myData
struct_message myData;

// ===== PID =====
PID pidpo(&currPose, &Delta_ang, &Setpo, Kppo, Kipo, Kdpo, DIRECT);
PID pidAngle(&Angle, &PWM, &Setpoint, Kp, Ki, Kd, DIRECT);

// ===== TIME =====
unsigned long prevTime;
float angle = 0;

// ===== ENCODER ISR =====
void IRAM_ATTR readEncoder() {
  if (digitalRead(ENCB)) pos++;
  else pos--;
}

void setup() {
  Serial.begin(115200);

  WiFi.begin(ssid, password);

  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
  }

  Serial.println("\nWiFi connected");

  client.setServer(mqtt_server, 1883);
  


  pinMode(ENCA, INPUT);
  pinMode(ENCB, INPUT);
  attachInterrupt(digitalPinToInterrupt(ENCA), readEncoder, RISING);

  pinMode(LT, OUTPUT);
  pinMode(LB, OUTPUT);
  pinMode(RT, OUTPUT);
  pinMode(RB, OUTPUT);

  WiFi.mode(WIFI_STA);

  // Init ESP-NOW
  if (esp_now_init() != ESP_OK) {
    Serial.println("Error initializing ESP-NOW");
    return;
  }

  // MPU init
  Wire.begin(4, 5, 100000);
  Wire.beginTransmission(0x68);
  Wire.write(0x6B);
  Wire.write(0);
  Wire.endTransmission(true);
  mpu.initialize();

  prevTime = micros();

  pidAngle.SetMode(AUTOMATIC);
  pidAngle.SetOutputLimits(-255, 255);
  pidAngle.SetSampleTime(5);
  pidpo.SetMode(AUTOMATIC);
  pidpo.SetOutputLimits(-10, 10);
  pidpo.SetSampleTime(30);

  if (esp_now_init() != ESP_OK) {
    Serial.println("Error initializing ESP-NOW");
    return;
  }

}

void loop() {



  // ===== อ่าน MPU =====
  int16_t ax, ay, az, gx, gy, gz;
  mpu.getAcceleration(&ax, &ay, &az);
  mpu.getRotation(&gx, &gy, &gz);

  float Ax = ax / 16384.0;
  float Ay = ay / 16384.0;
  float Az = az / 16384.0;
  float Gy = gy / 131.0;

  unsigned long currTime = micros();
  float dt = (currTime - prevTime) / 1000000.0;
  prevTime = currTime;

  float pitch_acc = atan(-Ax / sqrt(Ay * Ay + Az * Az)) * 180.0 / PI;

  // complementary filter
  angle = 0.98 * (angle + Gy * dt) + 0.02 * pitch_acc;
  Angle = angle;

  // ===== RPM =====
   currPos = pos;
  float speed = (currPos - prevPos) / dt;
  rpm = (speed / PPR) * 60.0;
  prevPos = currPos;

  // ===== CONTROL =====
  if (Angle > -25 && Angle < 25) {

    if(myData.EN_lock == 1){

    currPose = currPos/330;
    pidpo.Compute();

    }else{ 

      Setpo = currPos/330;
      Delta_ang = myData.y;

    }
    Setpoint = -5.68 - Delta_ang; // setpoint- DELTA_ang
    pidAngle.Compute();
    

    // ใส่ Kc แบบ KidBright
    PWM = (PWM + (Kc * rpm));

    // limit
    if (PWM > 255) PWM = 255;
    if (PWM < -255) PWM = -255;

    int pwmVal = abs(PWM);

    if (PWM > 0) {
      digitalWrite(LB, LOW);
      digitalWrite(RB, LOW);
      analogWrite(LT, (pwmVal-myData.x));
      analogWrite(RT,(pwmVal+myData.x));
    } else {
      digitalWrite(LT, LOW);
      digitalWrite(RT, LOW);
      analogWrite(LB, (pwmVal+myData.x));
      analogWrite(RB, (pwmVal-myData.x));
    }

  } else {
    analogWrite(LT, 0);
    analogWrite(RT, 0);

    analogWrite(LB, 0);
    analogWrite(RB, 0);
  }

  Serial.print("Angle: ");
  Serial.print(Angle);
  Serial.print(" | RPM: ");
  Serial.print(rpm);
  Serial.print(" | PWM: ");
  Serial.print(PWM);
  Serial.print(" | po: ");
  Serial.print(currPose);
  Serial.print(" | setpoint: ");
  Serial.print(Setpoint);
  Serial.print(" | ofset: ");
  Serial.print(myData.x);
  Serial.print(" | ofsetfront: ");
  Serial.print(myData.y);
  Serial.print(" | kd: ");
  Serial.println(Kd);



}


 

