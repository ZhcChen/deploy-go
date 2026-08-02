//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'setup_status_response.g.dart';

/// SetupStatusResponse
///
/// Properties:
/// * [setupEnabled]
/// * [setupRequired]
@BuiltValue()
abstract class SetupStatusResponse implements Built<SetupStatusResponse, SetupStatusResponseBuilder> {
  @BuiltValueField(wireName: r'setup_enabled')
  bool get setupEnabled;

  @BuiltValueField(wireName: r'setup_required')
  bool get setupRequired;

  SetupStatusResponse._();

  factory SetupStatusResponse([void updates(SetupStatusResponseBuilder b)]) = _$SetupStatusResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(SetupStatusResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<SetupStatusResponse> get serializer => _$SetupStatusResponseSerializer();
}

class _$SetupStatusResponseSerializer implements PrimitiveSerializer<SetupStatusResponse> {
  @override
  final Iterable<Type> types = const [SetupStatusResponse, _$SetupStatusResponse];

  @override
  final String wireName = r'SetupStatusResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    SetupStatusResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'setup_enabled';
    yield serializers.serialize(
      object.setupEnabled,
      specifiedType: const FullType(bool),
    );
    yield r'setup_required';
    yield serializers.serialize(
      object.setupRequired,
      specifiedType: const FullType(bool),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    SetupStatusResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required SetupStatusResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'setup_enabled':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.setupEnabled = valueDes;
          break;
        case r'setup_required':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(bool),
          ) as bool;
          result.setupRequired = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  SetupStatusResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = SetupStatusResponseBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
