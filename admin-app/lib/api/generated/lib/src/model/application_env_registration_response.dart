//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_env_registration_response.g.dart';

/// ApplicationEnvRegistrationResponse
///
/// Properties:
/// * [created]
@BuiltValue()
abstract class ApplicationEnvRegistrationResponse implements Built<ApplicationEnvRegistrationResponse, ApplicationEnvRegistrationResponseBuilder> {
  @BuiltValueField(wireName: r'created')
  BuiltList<String> get created;

  ApplicationEnvRegistrationResponse._();

  factory ApplicationEnvRegistrationResponse([void updates(ApplicationEnvRegistrationResponseBuilder b)]) = _$ApplicationEnvRegistrationResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationEnvRegistrationResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationEnvRegistrationResponse> get serializer => _$ApplicationEnvRegistrationResponseSerializer();
}

class _$ApplicationEnvRegistrationResponseSerializer implements PrimitiveSerializer<ApplicationEnvRegistrationResponse> {
  @override
  final Iterable<Type> types = const [ApplicationEnvRegistrationResponse, _$ApplicationEnvRegistrationResponse];

  @override
  final String wireName = r'ApplicationEnvRegistrationResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationEnvRegistrationResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'created';
    yield serializers.serialize(
      object.created,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationEnvRegistrationResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationEnvRegistrationResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'created':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.created.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationEnvRegistrationResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationEnvRegistrationResponseBuilder();
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
