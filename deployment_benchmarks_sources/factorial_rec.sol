pragma solidity ^0.8.0;

contract benchmark {
    uint256 value;
    constructor() {
      value = fac(57);
    }

    function get_calculation() public view returns (uint256) {
        return value;
    }

    function fac(uint256 n) internal returns (uint256) {
        if (n <= 1) {
            return 1;
        }
        return n * fac(n - 1);
    }
}

