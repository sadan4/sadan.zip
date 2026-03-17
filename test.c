#include <Windows.h>
#include <handleapi.h>
#include <stdio.h>
#include <synchapi.h>

int main() {
  STARTUPINFOA si = {.cb = sizeof(si)};
  PROCESS_INFORMATION pi = {0};
  int c =
      CreateProcessA("node.exe", "-c \"asd.123();\"",
                     NULL, NULL, TRUE, 0, NULL, NULL, &si, &pi);
  if (!c) {
    printf("CreateProcessA failed with error %lu\n", GetLastError());
    return 1;
  }

  WaitForSingleObject(pi.hProcess, INFINITE);

  CloseHandle(pi.hProcess);
  CloseHandle(pi.hThread);
  printf("Process completed successfully\n");
  return 0;
}